//! RunEngineService implementation (bd-w14j.2)
//!
//! Provides gRPC interface for the Bluesky-inspired RunEngine.
//! Enables declarative plan execution with pause/resume/abort capabilities.

use crate::grpc::proto::{
    run_engine_service_server::RunEngineService, AbortPlanRequest, AbortPlanResponse, EngineStatus,
    GetEngineStatusRequest, HaltEngineRequest, HaltEngineResponse, ListPlanTypesRequest,
    ListPlanTypesResponse, PauseEngineRequest, PauseEngineResponse, PlanTypeInfo, QueuePlanRequest,
    QueuePlanResponse, ResumeEngineRequest, ResumeEngineResponse, StartEngineRequest,
    StartEngineResponse, StreamDocumentsRequest,
};
use daq_experiment::run_engine::RunEngine;
use daq_experiment::Document; // Re-exported from daq_core
use futures::StreamExt; // For .filter_map() with async
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tonic::{Request, Response, Status};

/// RunEngine gRPC service implementation.
///
/// Wraps the domain RunEngine and exposes its capabilities over gRPC.
///
/// Performance optimization (bd-p3k0): Converts domain documents to proto ONCE
/// and broadcasts to all clients, avoiding O(N×M) conversions for N clients and M events.
///
/// Observability (bd-f9hn): Tracks active streams and emits structured tracing events
/// for monitoring document throughput, lag events, and conversion performance.
pub struct RunEngineServiceImpl {
    engine: Arc<RunEngine>,
    /// Proto document broadcast (converted once, shared across all clients)
    proto_doc_sender: tokio::sync::broadcast::Sender<Arc<crate::grpc::proto::Document>>,
    /// Active stream count for observability (bd-f9hn)
    active_streams: Arc<AtomicU64>,
}

impl RunEngineServiceImpl {
    /// Construct a new RunEngine service.
    ///
    /// Spawns a background task that converts domain documents to proto and broadcasts
    /// them to all gRPC clients. This ensures O(M) conversions instead of O(N×M).
    pub fn new(engine: Arc<RunEngine>) -> Self {
        // Create proto document broadcast channel
        let (proto_doc_sender, _) = tokio::sync::broadcast::channel(1024);

        // Create observability metrics
        let active_streams = Arc::new(AtomicU64::new(0));

        // Spawn converter task that subscribes to domain stream and broadcasts proto
        let engine_clone = engine.clone();
        let proto_sender_clone = proto_doc_sender.clone();
        tokio::spawn(async move {
            let mut domain_rx = engine_clone.subscribe();
            let mut total_converted = 0u64;

            loop {
                match domain_rx.recv().await {
                    Ok(domain_doc) => {
                        // Convert domain → proto (ONCE for all clients)
                        let start = std::time::Instant::now();
                        match domain_to_proto_document(domain_doc) {
                            Ok(Some(proto_doc)) => {
                                let conversion_micros = start.elapsed().as_micros();
                                total_converted += 1;

                                // Broadcast Arc to avoid cloning the proto doc for each client
                                let subscriber_count = proto_sender_clone.receiver_count();
                                let send_result = proto_sender_clone.send(Arc::new(proto_doc));

                                // Observability: Log throughput periodically
                                if total_converted % 1000 == 0 {
                                    tracing::info!(
                                        total_converted,
                                        subscriber_count,
                                        conversion_micros,
                                        "Document converter throughput"
                                    );
                                }

                                // Warn if no subscribers (documents being dropped)
                                if send_result.is_err() || subscriber_count == 0 {
                                    tracing::debug!(
                                        total_converted,
                                        "Document broadcast has no subscribers"
                                    );
                                }
                            }
                            Ok(None) => {
                                // Skip documents that don't convert (e.g., Manifest)
                                tracing::trace!("Skipped non-convertible document (e.g., Manifest)");
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    total_converted,
                                    "Failed to convert domain document to proto"
                                );
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            skipped,
                            total_converted,
                            "Converter task lagged, skipped domain documents"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!(
                            total_converted,
                            "RunEngine document stream closed, stopping converter task"
                        );
                        break;
                    }
                }
            }
        });

        Self {
            engine,
            proto_doc_sender,
            active_streams,
        }
    }
}

impl Clone for RunEngineServiceImpl {
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            proto_doc_sender: self.proto_doc_sender.clone(),
            active_streams: self.active_streams.clone(),
        }
    }
}

#[tonic::async_trait]
impl RunEngineService for RunEngineServiceImpl {
    async fn list_plan_types(
        &self,
        _request: Request<ListPlanTypesRequest>,
    ) -> Result<Response<ListPlanTypesResponse>, Status> {
        use crate::grpc::proto::PlanTypeSummary;

        // Return hardcoded list of available plan types
        let plan_types = vec![
            PlanTypeSummary {
                type_id: "count".to_string(),
                display_name: "Count".to_string(),
                description: "Repeated measurements at current position".to_string(),
                categories: vec!["0d".to_string()],
            },
            PlanTypeSummary {
                type_id: "line_scan".to_string(),
                display_name: "Line Scan".to_string(),
                description: "1D linear scan along a motor axis".to_string(),
                categories: vec!["scanning".to_string(), "1d".to_string()],
            },
            PlanTypeSummary {
                type_id: "grid_scan".to_string(),
                display_name: "Grid Scan".to_string(),
                description: "2D grid scan over two motor axes".to_string(),
                categories: vec!["scanning".to_string(), "2d".to_string()],
            },
        ];

        Ok(Response::new(ListPlanTypesResponse { plan_types }))
    }

    async fn get_plan_type_info(
        &self,
        _request: Request<crate::grpc::proto::GetPlanTypeInfoRequest>,
    ) -> Result<Response<PlanTypeInfo>, Status> {
        Err(Status::unimplemented("get_plan_type_info not yet implemented"))
    }

    async fn queue_plan(
        &self,
        request: Request<QueuePlanRequest>,
    ) -> Result<Response<QueuePlanResponse>, Status> {
        let req = request.get_ref();

        // Create plan from request parameters
        let plan = create_plan_from_request(req)
            .map_err(|e| Status::invalid_argument(format!("Failed to create plan: {}", e)))?;

        // Queue the plan
        let run_uid = if req.metadata.is_empty() {
            self.engine.queue(plan).await
        } else {
            self.engine.queue_with_metadata(plan, req.metadata.clone()).await
        };

        let queue_len = self.engine.queue_len().await;

        Ok(Response::new(QueuePlanResponse {
            success: true,
            run_uid,
            error_message: String::new(),
            queue_position: queue_len as u32,
        }))
    }

    async fn start_engine(
        &self,
        _request: Request<StartEngineRequest>,
    ) -> Result<Response<StartEngineResponse>, Status> {
        // Start the engine (spawns background task)
        self.engine.start().await
            .map_err(|e| Status::internal(format!("Failed to start engine: {}", e)))?;

        Ok(Response::new(StartEngineResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn pause_engine(
        &self,
        _request: Request<PauseEngineRequest>,
    ) -> Result<Response<PauseEngineResponse>, Status> {
        match self.engine.pause().await {
            Ok(_) => Ok(Response::new(PauseEngineResponse {
                success: true,
                paused_at: "checkpoint".to_string(),
            })),
            Err(e) => Err(Status::internal(format!("Failed to pause engine: {}", e))),
        }
    }

    async fn resume_engine(
        &self,
        _request: Request<ResumeEngineRequest>,
    ) -> Result<Response<ResumeEngineResponse>, Status> {
        self.engine.resume().await
            .map_err(|e| Status::internal(format!("Failed to resume engine: {}", e)))?;

        Ok(Response::new(ResumeEngineResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn abort_plan(
        &self,
        request: Request<AbortPlanRequest>,
    ) -> Result<Response<AbortPlanResponse>, Status> {
        let _req = request.into_inner();

        // TODO: Support aborting specific run_uid (currently aborts current run only)
        // For now, ignore _req.run_uid and abort the currently executing plan
        self.engine.abort("user requested abort via gRPC").await
            .map_err(|e| Status::internal(format!("Failed to abort plan: {}", e)))?;

        Ok(Response::new(AbortPlanResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn halt_engine(
        &self,
        _request: Request<HaltEngineRequest>,
    ) -> Result<Response<HaltEngineResponse>, Status> {
        self.engine.halt().await
            .map_err(|e| Status::internal(format!("Failed to halt engine: {}", e)))?;

        Ok(Response::new(HaltEngineResponse {
            halted: true,
            message: "Engine halted successfully".to_string(),
        }))
    }

    async fn get_engine_status(
        &self,
        _request: Request<GetEngineStatusRequest>,
    ) -> Result<Response<EngineStatus>, Status> {
        use crate::grpc::proto::EngineState as ProtoEngineState;
        use daq_experiment::run_engine::EngineState as DomainEngineState;

        let domain_state = self.engine.state().await;
        let queue_len = self.engine.queue_len().await as u32;

        let proto_state = match domain_state {
            DomainEngineState::Idle => ProtoEngineState::EngineIdle,
            DomainEngineState::Running => ProtoEngineState::EngineRunning,
            DomainEngineState::Paused => ProtoEngineState::EnginePaused,
            DomainEngineState::Aborting => ProtoEngineState::EngineAborting,
        };

        Ok(Response::new(EngineStatus {
            state: proto_state as i32,
            current_run_uid: None,
            current_plan_type: None,
            current_event_number: None,
            total_events_expected: None,
            queued_plans: queue_len,
            run_start_ns: 0, // TODO: Track run start time
            elapsed_ns: 0,   // TODO: Track elapsed time
        }))
    }

    type StreamDocumentsStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<crate::grpc::proto::Document, Status>> + Send>>;

    async fn stream_documents(
        &self,
        request: Request<StreamDocumentsRequest>,
    ) -> Result<Response<Self::StreamDocumentsStream>, Status> {
        // Subscribe to proto document broadcast (already converted by background task)
        // Performance: O(M) conversions instead of O(N×M) for N clients, M events
        let proto_rx = self.proto_doc_sender.subscribe();

        // Extract filters from request
        let req = request.into_inner();
        let run_uid_filter = req.run_uid.filter(|s| !s.is_empty());

        // Wrap doc_types filter in Arc to avoid cloning Vec on every document
        let doc_types_filter: Option<Arc<Vec<i32>>> = if req.doc_types.is_empty() {
            None
        } else {
            Some(Arc::new(req.doc_types))
        };

        // Maintain descriptor_uid → run_uid mapping for Event filtering
        // Events only have descriptor_uid, need to look up run_uid
        let descriptor_to_run_map = Arc::new(tokio::sync::Mutex::new(
            std::collections::HashMap::<String, String>::new()
        ));

        // Observability: Track active streams (bd-f9hn)
        let active_streams = self.active_streams.clone();
        let stream_count = active_streams.fetch_add(1, Ordering::Relaxed) + 1;
        tracing::info!(
            active_streams = stream_count,
            run_uid_filter = ?run_uid_filter,
            doc_types_filter = ?doc_types_filter,
            "New document stream client connected"
        );

        // Metrics: Track filter matches and rejections per client
        let docs_received = Arc::new(AtomicU64::new(0));
        let docs_filtered = Arc::new(AtomicU64::new(0));
        let docs_sent = Arc::new(AtomicU64::new(0));
        let lag_events = Arc::new(AtomicU64::new(0));

        // Clone metrics for use outside closure
        let docs_received_outer = docs_received.clone();
        let docs_sent_outer = docs_sent.clone();
        let docs_filtered_outer = docs_filtered.clone();
        let lag_events_outer = lag_events.clone();

        // Apply client-specific filters to the shared proto stream
        let stream = BroadcastStream::new(proto_rx).filter_map(move |result| {
            let run_uid_filter = run_uid_filter.clone();
            let doc_types_filter = doc_types_filter.clone(); // Arc clone, cheap
            let descriptor_map = descriptor_to_run_map.clone();
            let docs_received = docs_received.clone();
            let docs_filtered = docs_filtered.clone();
            let docs_sent = docs_sent.clone();
            let lag_events = lag_events.clone();

            async move {
                match result {
                    Ok(proto_doc_arc) => {
                        // Observability: Track received documents
                        let received = docs_received.fetch_add(1, Ordering::Relaxed) + 1;

                        // Document already converted to proto by background task
                        // Apply run_uid filter
                        if let Some(ref filter_uid) = run_uid_filter {
                            // Extract run_uid from document based on type
                            let doc_run_uid = match &proto_doc_arc.payload {
                                Some(crate::grpc::proto::document::Payload::Start(s)) => {
                                    Some(s.run_uid.clone())
                                }
                                Some(crate::grpc::proto::document::Payload::Stop(s)) => {
                                    Some(s.run_uid.clone())
                                }
                                Some(crate::grpc::proto::document::Payload::Descriptor(d)) => {
                                    // Record descriptor_uid → run_uid mapping
                                    let mut map = descriptor_map.lock().await;
                                    map.insert(d.descriptor_uid.clone(), d.run_uid.clone());
                                    Some(d.run_uid.clone())
                                }
                                Some(crate::grpc::proto::document::Payload::Event(e)) => {
                                    // Look up run_uid via descriptor_uid
                                    let map = descriptor_map.lock().await;
                                    map.get(&e.descriptor_uid).cloned()
                                }
                                None => None,
                            };

                            if let Some(uid) = doc_run_uid {
                                if uid != *filter_uid {
                                    docs_filtered.fetch_add(1, Ordering::Relaxed);
                                    return None; // Skip - different run
                                }
                            } else if matches!(&proto_doc_arc.payload, Some(crate::grpc::proto::document::Payload::Event(_))) {
                                // Event with unknown descriptor - drop when filter active
                                docs_filtered.fetch_add(1, Ordering::Relaxed);
                                tracing::debug!(
                                    "Dropping Event with unknown descriptor_uid (run_uid filter active)"
                                );
                                return None;
                            }
                        }

                        // Apply doc_types filter
                        if let Some(ref filter_types) = doc_types_filter {
                            if !filter_types.contains(&proto_doc_arc.doc_type) {
                                docs_filtered.fetch_add(1, Ordering::Relaxed);
                                return None; // Skip - type not in filter
                            }
                        }

                        // Observability: Track sent documents and log periodically
                        let sent = docs_sent.fetch_add(1, Ordering::Relaxed) + 1;
                        if sent % 100 == 0 {
                            let filtered = docs_filtered.load(Ordering::Relaxed);
                            let filter_rate = if received > 0 {
                                (sent as f64 / received as f64) * 100.0
                            } else {
                                0.0
                            };
                            tracing::info!(
                                docs_received = received,
                                docs_sent = sent,
                                docs_filtered = filtered,
                                filter_match_rate_percent = format!("{:.1}", filter_rate),
                                "Client stream metrics"
                            );
                        }

                        // Clone the proto doc for this client (Arc::deref + clone)
                        // This is cheaper than re-converting from domain
                        Some(Ok((*proto_doc_arc).clone()))
                    }
                    Err(BroadcastStreamRecvError::Lagged(skipped)) => {
                        // Receiver fell behind - log and continue without terminating stream
                        let lag_count = lag_events.fetch_add(1, Ordering::Relaxed) + 1;
                        let received = docs_received.load(Ordering::Relaxed);
                        let sent = docs_sent.load(Ordering::Relaxed);
                        tracing::warn!(
                            skipped,
                            lag_count,
                            docs_received = received,
                            docs_sent = sent,
                            "Document stream lagged: client too slow, skipped messages"
                        );
                        None // Skip, don't terminate
                    }
                    // Note: BroadcastStreamRecvError does not have a Closed variant
                    // The stream ends with None when the sender is dropped
                    // This exhaustive match ensures we handle all actual error cases
                }
            }
        });

        // Wrap stream to decrement active_streams on drop
        let active_streams_cleanup = self.active_streams.clone();
        let wrapped_stream = StreamWithCleanup {
            inner: Box::pin(stream),
            active_streams: active_streams_cleanup,
            docs_received: docs_received_outer,
            docs_sent: docs_sent_outer,
            docs_filtered: docs_filtered_outer,
            lag_events: lag_events_outer,
        };

        Ok(Response::new(Box::pin(wrapped_stream)))
    }
}

/// Wrapper to decrement active_streams counter when stream is dropped
struct StreamWithCleanup<S> {
    inner: std::pin::Pin<Box<S>>,
    active_streams: Arc<AtomicU64>,
    docs_received: Arc<AtomicU64>,
    docs_sent: Arc<AtomicU64>,
    docs_filtered: Arc<AtomicU64>,
    lag_events: Arc<AtomicU64>,
}

impl<S> Drop for StreamWithCleanup<S> {
    fn drop(&mut self) {
        let remaining = self.active_streams.fetch_sub(1, Ordering::Relaxed) - 1;
        let received = self.docs_received.load(Ordering::Relaxed);
        let sent = self.docs_sent.load(Ordering::Relaxed);
        let filtered = self.docs_filtered.load(Ordering::Relaxed);
        let lags = self.lag_events.load(Ordering::Relaxed);

        tracing::info!(
            active_streams = remaining,
            docs_received = received,
            docs_sent = sent,
            docs_filtered = filtered,
            lag_events = lags,
            "Document stream client disconnected"
        );
    }
}

impl<S> tokio_stream::Stream for StreamWithCleanup<S>
where
    S: tokio_stream::Stream,
{
    type Item = S::Item;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Create a Plan from QueuePlanRequest parameters
fn create_plan_from_request(req: &QueuePlanRequest) -> Result<Box<dyn daq_experiment::plans::Plan>, String> {
    use daq_experiment::plans::{Count, GridScan, LineScan};

    match req.plan_type.as_str() {
        "count" => {
            // Parse count parameters
            let num_points = req.parameters.get("num_points")
                .ok_or("Missing parameter: num_points")?
                .parse::<usize>()
                .map_err(|e| format!("Invalid num_points: {}", e))?;

            // Validate parameters
            if num_points == 0 {
                return Err("num_points must be > 0".to_string());
            }
            if num_points > 10_000_000 {
                return Err("num_points must be <= 10,000,000 to prevent resource exhaustion".to_string());
            }

            let mut plan = Count::new(num_points);

            // Optional detector
            if let Some(detector) = req.device_mapping.get("detector") {
                if detector.is_empty() {
                    return Err("detector device name cannot be empty".to_string());
                }
                plan = plan.with_detector(detector);
            }

            // Optional delay
            if let Some(delay_str) = req.parameters.get("delay") {
                let delay = delay_str.parse::<f64>()
                    .map_err(|e| format!("Invalid delay: {}", e))?;
                if !delay.is_finite() {
                    return Err("delay must be a finite number (not NaN or infinity)".to_string());
                }
                if delay < 0.0 {
                    return Err("delay must be >= 0".to_string());
                }
                plan = plan.with_delay(delay);
            }

            Ok(Box::new(plan))
        }
        "line_scan" => {
            // Parse line scan parameters
            let start = req.parameters.get("start")
                .ok_or("Missing parameter: start")?
                .parse::<f64>()
                .map_err(|e| format!("Invalid start: {}", e))?;

            let end = req.parameters.get("end")
                .ok_or("Missing parameter: end")?
                .parse::<f64>()
                .map_err(|e| format!("Invalid end: {}", e))?;

            let num_points = req.parameters.get("num_points")
                .ok_or("Missing parameter: num_points")?
                .parse::<usize>()
                .map_err(|e| format!("Invalid num_points: {}", e))?;

            let motor = req.device_mapping.get("motor")
                .ok_or("Missing device mapping: motor")?;

            // Validate parameters
            if !start.is_finite() {
                return Err("start must be a finite number (not NaN or infinity)".to_string());
            }
            if !end.is_finite() {
                return Err("end must be a finite number (not NaN or infinity)".to_string());
            }
            if num_points == 0 {
                return Err("num_points must be > 0".to_string());
            }
            if num_points > 10_000_000 {
                return Err("num_points must be <= 10,000,000 to prevent resource exhaustion".to_string());
            }
            if start == end {
                return Err("start and end must be different for line scan".to_string());
            }
            if motor.is_empty() {
                return Err("motor device name cannot be empty".to_string());
            }

            let mut plan = LineScan::new(motor, start, end, num_points);

            // Optional detector
            if let Some(detector) = req.device_mapping.get("detector") {
                if detector.is_empty() {
                    return Err("detector device name cannot be empty".to_string());
                }
                plan = plan.with_detector(detector);
            }

            // Optional settle time
            if let Some(settle_str) = req.parameters.get("settle_time") {
                let settle = settle_str.parse::<f64>()
                    .map_err(|e| format!("Invalid settle_time: {}", e))?;
                if !settle.is_finite() {
                    return Err("settle_time must be a finite number (not NaN or infinity)".to_string());
                }
                if settle < 0.0 {
                    return Err("settle_time must be >= 0".to_string());
                }
                plan = plan.with_settle_time(settle);
            }

            Ok(Box::new(plan))
        }
        "grid_scan" => {
            // Parse grid scan parameters
            let x_start = req.parameters.get("x_start")
                .ok_or("Missing parameter: x_start")?
                .parse::<f64>()
                .map_err(|e| format!("Invalid x_start: {}", e))?;

            let x_end = req.parameters.get("x_end")
                .ok_or("Missing parameter: x_end")?
                .parse::<f64>()
                .map_err(|e| format!("Invalid x_end: {}", e))?;

            let x_points = req.parameters.get("x_points")
                .ok_or("Missing parameter: x_points")?
                .parse::<usize>()
                .map_err(|e| format!("Invalid x_points: {}", e))?;

            let y_start = req.parameters.get("y_start")
                .ok_or("Missing parameter: y_start")?
                .parse::<f64>()
                .map_err(|e| format!("Invalid y_start: {}", e))?;

            let y_end = req.parameters.get("y_end")
                .ok_or("Missing parameter: y_end")?
                .parse::<f64>()
                .map_err(|e| format!("Invalid y_end: {}", e))?;

            let y_points = req.parameters.get("y_points")
                .ok_or("Missing parameter: y_points")?
                .parse::<usize>()
                .map_err(|e| format!("Invalid y_points: {}", e))?;

            let x_motor = req.device_mapping.get("x_motor")
                .ok_or("Missing device mapping: x_motor")?;

            let y_motor = req.device_mapping.get("y_motor")
                .ok_or("Missing device mapping: y_motor")?;

            // Validate parameters
            if !x_start.is_finite() {
                return Err("x_start must be a finite number (not NaN or infinity)".to_string());
            }
            if !x_end.is_finite() {
                return Err("x_end must be a finite number (not NaN or infinity)".to_string());
            }
            if !y_start.is_finite() {
                return Err("y_start must be a finite number (not NaN or infinity)".to_string());
            }
            if !y_end.is_finite() {
                return Err("y_end must be a finite number (not NaN or infinity)".to_string());
            }
            if x_points == 0 {
                return Err("x_points must be > 0".to_string());
            }
            if x_points > 100_000 {
                return Err("x_points must be <= 100,000 to prevent resource exhaustion".to_string());
            }
            if y_points == 0 {
                return Err("y_points must be > 0".to_string());
            }
            if y_points > 100_000 {
                return Err("y_points must be <= 100,000 to prevent resource exhaustion".to_string());
            }
            if x_start == x_end {
                return Err("x_start and x_end must be different for grid scan".to_string());
            }
            if y_start == y_end {
                return Err("y_start and y_end must be different for grid scan".to_string());
            }
            if x_motor.is_empty() {
                return Err("x_motor device name cannot be empty".to_string());
            }
            if y_motor.is_empty() {
                return Err("y_motor device name cannot be empty".to_string());
            }
            if x_motor == y_motor {
                return Err("x_motor and y_motor must be different".to_string());
            }

            // Note: GridScan takes (outer/slow, inner/fast) axes
            // Convention: y is outer (slow), x is inner (fast)
            let mut plan = GridScan::new(
                y_motor,
                y_start,
                y_end,
                y_points,
                x_motor,
                x_start,
                x_end,
                x_points,
            );

            // Optional detector
            if let Some(detector) = req.device_mapping.get("detector") {
                if detector.is_empty() {
                    return Err("detector device name cannot be empty".to_string());
                }
                plan = plan.with_detector(detector);
            }

            // Optional snake scanning
            if let Some(snake_str) = req.parameters.get("snake") {
                let snake = snake_str.parse::<bool>()
                    .map_err(|e| format!("Invalid snake: {}", e))?;
                plan = plan.with_snake(snake);
            }

            Ok(Box::new(plan))
        }
        _ => Err(format!("Unknown plan type: {}", req.plan_type)),
    }
}

/// Convert domain Document to proto Document
/// Returns Ok(None) for documents that have no proto equivalent (e.g., Manifest)
fn domain_to_proto_document(
    doc: Document,
) -> Result<Option<crate::grpc::proto::Document>, String> {
    use crate::grpc::proto::{
        DataKey as ProtoDataKey, DescriptorDocument, Document as ProtoDocument,
        DocumentType as ProtoDocType, EventDocument, StartDocument, StopDocument,
    };
    use daq_experiment::Document as DomainDoc;

    let (doc_type, uid, timestamp_ns, payload) = match doc {
        DomainDoc::Start(start) => {
            let proto_start = StartDocument {
                run_uid: start.uid.clone(),
                plan_type: start.plan_type.clone(),
                plan_name: start.plan_name.clone(),
                plan_args: start.plan_args.clone(),
                metadata: start.metadata.clone(),
                hints: start.hints.clone(),
                time_ns: start.time_ns,
            };
            (
                ProtoDocType::DocStart as i32,
                start.uid,
                start.time_ns,
                Some(crate::grpc::proto::document::Payload::Start(proto_start)),
            )
        }
        DomainDoc::Stop(stop) => {
            let proto_stop = StopDocument {
                run_uid: stop.run_uid.clone(),
                exit_status: stop.exit_status.clone(),
                reason: stop.reason.clone(),
                time_ns: stop.time_ns,
                num_events: stop.num_events,
            };
            (
                ProtoDocType::DocStop as i32,
                stop.uid,
                stop.time_ns,
                Some(crate::grpc::proto::document::Payload::Stop(proto_stop)),
            )
        }
        DomainDoc::Event(event) => {
            let proto_event = EventDocument {
                descriptor_uid: event.descriptor_uid.clone(),
                seq_num: event.seq_num,
                time_ns: event.time_ns,
                data: event.data.clone(),
                timestamps: event.timestamps.clone(),
                bulk_data: std::collections::HashMap::new(), // TODO: Bulk data support
            };
            (
                ProtoDocType::DocEvent as i32,
                event.uid,
                event.time_ns,
                Some(crate::grpc::proto::document::Payload::Event(proto_event)),
            )
        }
        DomainDoc::Descriptor(desc) => {
            // Convert DataKey HashMap
            let proto_data_keys = desc
                .data_keys
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        ProtoDataKey {
                            dtype: v.dtype,
                            shape: v.shape,
                            source: v.source,
                            units: v.units,
                            precision: v.precision,
                        },
                    )
                })
                .collect();

            let proto_desc = DescriptorDocument {
                run_uid: desc.run_uid.clone(),
                descriptor_uid: desc.uid.clone(),
                name: desc.name.clone(),
                data_keys: proto_data_keys,
                configuration: desc.configuration.clone(),
            };
            (
                ProtoDocType::DocDescriptor as i32,
                desc.uid,
                desc.time_ns,
                Some(crate::grpc::proto::document::Payload::Descriptor(proto_desc)),
            )
        }
        DomainDoc::Manifest(_manifest) => {
            // Manifest has no proto equivalent - skip gracefully
            tracing::debug!("Skipping Manifest document (no proto mapping)");
            return Ok(None);
        }
    };

    Ok(Some(ProtoDocument {
        doc_type,
        uid,
        timestamp_ns,
        payload,
    }))
}
