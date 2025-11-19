# Phase 3: Network Layer (Weeks 5-6)

## Phase 3: Network Layer Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Separate UI from Core with gRPC/WebSocket communication.

  OBJECTIVE: Enable remote control and crash-resilient operation.
  TIMELINE: Weeks 5-6
  PARALLELIZABLE: Tasks G, H can overlap with I

  SUCCESS CRITERIA:
  - gRPC server running on rust-daq-core
  - Client can upload and execute Rhai scripts remotely
  - UI disconnect doesn't stop running experiment
  - < 10ms latency for script upload (< 1KB script)
  - WebSocket stream for real-time status updates

## Task G: API Definition with Protocol Buffers
type: task
priority: P0
parent: bd-oq51.3
description: |
  Define gRPC service interface using Protocol Buffers.

  CREATE: src/network/proto/daq.proto

  REFERENCE IMPLEMENTATION:
  ```protobuf
  syntax = "proto3";
  package daq;

  service ControlService {
    // Script Management
    rpc UploadScript (UploadRequest) returns (UploadResponse);
    rpc StartScript (StartRequest) returns (StartResponse);
    rpc StopScript (StopRequest) returns (StopResponse);
    rpc GetScriptStatus (StatusRequest) returns (ScriptStatus);

    // Live Data Streaming
    rpc StreamStatus (StatusRequest) returns (stream SystemStatus);
    rpc StreamMeasurements (MeasurementRequest) returns (stream DataPoint);
  }

  message UploadRequest {
    string script_content = 1;
    string name = 2;
    map<string, string> metadata = 3;
  }

  message UploadResponse {
    string script_id = 1;
    bool success = 2;
    string error_message = 3;
  }

  message StartRequest {
    string script_id = 1;
  }

  message StartResponse {
    bool started = 1;
    string execution_id = 2;
  }

  message SystemStatus {
    string current_state = 1; // "IDLE", "RUNNING", "ERROR"
    double current_memory_usage_mb = 2;
    map<string, double> live_values = 3; // e.g., {"stage_x": 10.2}
    uint64 timestamp_ns = 4;
  }

  message DataPoint {
    string instrument = 1;
    oneof value {
      double scalar = 2;
      bytes image = 3; // Compressed image data
    }
    uint64 timestamp_ns = 4;
  }
  ```

  BUILD.RS CONFIGURATION:
  ```rust
  fn main() -> Result<(), Box<dyn std::error::Error>> {
      tonic_build::configure()
          .build_server(true)
          .build_client(true)
          .compile(&["src/network/proto/daq.proto"], &["src/network/proto"])?;
      Ok(())
  }
  ```

  DEPENDENCIES (Cargo.toml):
  ```toml
  [dependencies]
  tonic = "0.10"
  prost = "0.12"
  tokio-stream = "0.1"

  [build-dependencies]
  tonic-build = "0.10"
  ```

  ACCEPTANCE:
  - src/network/proto/daq.proto exists
  - build.rs generates Rust code from proto
  - ControlService trait available in src/network/daq.rs
  - cargo build succeeds

## Task H: gRPC Server Implementation
type: task
priority: P0
parent: bd-oq51.3
deps: bd-oq51.3.1
description: |
  Implement gRPC server that controls ScriptHost and hardware.

  CREATE: src/network/server.rs

  IMPLEMENTATION:
  ```rust
  use tonic::{transport::Server, Request, Response, Status};
  use crate::network::daq::{
      control_service_server::{ControlService, ControlServiceServer},
      UploadRequest, UploadResponse, StartRequest, StartResponse,
      SystemStatus,
  };
  use crate::scripting::ScriptHost;
  use std::sync::Arc;
  use tokio::sync::RwLock;

  pub struct DaqServer {
      script_host: Arc<RwLock<ScriptHost>>,
      scripts: Arc<RwLock<HashMap<String, String>>>,
      current_execution: Arc<RwLock<Option<String>>>,
  }

  #[tonic::async_trait]
  impl ControlService for DaqServer {
      async fn upload_script(
          &self,
          request: Request<UploadRequest>,
      ) -> Result<Response<UploadResponse>, Status> {
          let req = request.into_inner();
          let script_id = Uuid::new_v4().to_string();

          // Validate script compiles
          let engine = self.script_host.read().await;
          if let Err(e) = engine.validate_script(&req.script_content) {
              return Ok(Response::new(UploadResponse {
                  script_id: String::new(),
                  success: false,
                  error_message: format!("Script validation failed: {}", e),
              }));
          }

          // Store script
          self.scripts.write().await.insert(script_id.clone(), req.script_content);

          Ok(Response::new(UploadResponse {
              script_id,
              success: true,
              error_message: String::new(),
          }))
      }

      async fn start_script(
          &self,
          request: Request<StartRequest>,
      ) -> Result<Response<StartResponse>, Status> {
          let req = request.into_inner();
          let scripts = self.scripts.read().await;

          let script = scripts.get(&req.script_id)
              .ok_or_else(|| Status::not_found("Script not found"))?;

          // Execute in background task
          let script_clone = script.clone();
          let host_clone = self.script_host.clone();

          let execution_id = Uuid::new_v4().to_string();
          let exec_id_clone = execution_id.clone();

          tokio::spawn(async move {
              let host = host_clone.read().await;
              if let Err(e) = host.run_script(&script_clone) {
                  eprintln!("[{}] Script error: {}", exec_id_clone, e);
              }
          });

          Ok(Response::new(StartResponse {
              started: true,
              execution_id,
          }))
      }

      type StreamStatusStream = tokio_stream::wrappers::ReceiverStream<Result<SystemStatus, Status>>;

      async fn stream_status(
          &self,
          _request: Request<StatusRequest>,
      ) -> Result<Response<Self::StreamStatusStream>, Status> {
          let (tx, rx) = tokio::sync::mpsc::channel(100);

          // Background task: Send status updates every 100ms
          tokio::spawn(async move {
              let mut interval = tokio::time::interval(Duration::from_millis(100));
              loop {
                  interval.tick().await;
                  let status = SystemStatus {
                      current_state: "RUNNING".to_string(),
                      current_memory_usage_mb: 42.0, // TODO: real metrics
                      live_values: HashMap::new(),
                      timestamp_ns: SystemTime::now()
                          .duration_since(UNIX_EPOCH)
                          .unwrap()
                          .as_nanos() as u64,
                  };

                  if tx.send(Ok(status)).await.is_err() {
                      break; // Client disconnected
                  }
              }
          });

          Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
      }
  }

  pub async fn start_server(addr: SocketAddr) -> Result<()> {
      let server = DaqServer {
          script_host: Arc::new(RwLock::new(ScriptHost::new(Handle::current()))),
          scripts: Arc::new(RwLock::new(HashMap::new())),
          current_execution: Arc::new(RwLock::new(None)),
      };

      println!("DAQ gRPC server listening on {}", addr);

      Server::builder()
          .add_service(ControlServiceServer::new(server))
          .serve(addr)
          .await?;

      Ok(())
  }
  ```

  ACCEPTANCE:
  - Server starts on port 50051
  - UploadScript RPC validates and stores scripts
  - StartScript RPC executes script in background
  - StreamStatus sends updates every 100ms
  - Server survives script errors without crashing

## Task I: Client Prototype (Python)
type: task
priority: P0
parent: bd-oq51.3
deps: bd-oq51.3.2
description: |
  Build Python client to demonstrate remote control.

  CREATE: clients/python/daq_client.py

  DEPENDENCIES (requirements.txt):
  ```
  grpcio==1.59.0
  grpcio-tools==1.59.0
  ```

  IMPLEMENTATION:
  ```python
  import grpc
  from generated import daq_pb2, daq_pb2_grpc
  import time

  class DaqClient:
      def __init__(self, host='localhost', port=50051):
          self.channel = grpc.insecure_channel(f'{host}:{port}')
          self.stub = daq_pb2_grpc.ControlServiceStub(self.channel)

      def upload_script(self, script_content, name="experiment"):
          request = daq_pb2.UploadRequest(
              script_content=script_content,
              name=name
          )
          response = self.stub.UploadScript(request)

          if not response.success:
              raise RuntimeError(f"Upload failed: {response.error_message}")

          return response.script_id

      def start_script(self, script_id):
          request = daq_pb2.StartRequest(script_id=script_id)
          response = self.stub.StartScript(request)
          return response.execution_id

      def stream_status(self):
          request = daq_pb2.StatusRequest()
          for status in self.stub.StreamStatus(request):
              yield status

  # Example usage
  if __name__ == "__main__":
      client = DaqClient()

      # Upload experiment script
      script = """
      print("Hello from Rhai!");
      stage.move_abs(5.0);
      sleep(0.5);
      print("Movement complete");
      """

      script_id = client.upload_script(script)
      print(f"Uploaded script: {script_id}")

      # Start execution
      exec_id = client.start_script(script_id)
      print(f"Started execution: {exec_id}")

      # Monitor status
      print("Monitoring status (Ctrl+C to stop):")
      for status in client.stream_status():
          print(f"  State: {status.current_state}, Memory: {status.current_memory_usage_mb:.1f} MB")
          time.sleep(1)
  ```

  CODEGEN:
  ```bash
  python -m grpc_tools.protoc \
      -I../../src/network/proto \
      --python_out=generated \
      --grpc_python_out=generated \
      ../../src/network/proto/daq.proto
  ```

  ACCEPTANCE:
  - client.py can connect to server
  - upload_script() succeeds
  - start_script() triggers execution
  - stream_status() receives updates
  - Experiment runs end-to-end via remote client
