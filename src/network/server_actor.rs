use crate::messages::DaqCommand;
use crate::network::protocol::{
    ControlRequest, ControlResponse, Heartbeat, RequestType, ResponseStatus,
};
use crate::network::session::SessionManager;
use crate::core::{InstrumentCommand, ParameterValue};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{interval, timeout, Duration};
use uuid::Uuid;

const HEARTBEAT_INTERVAL_MS: u64 = 2000;
const SESSION_TIMEOUT_SECS: u64 = 6;
const READ_TIMEOUT_MS: u64 = 10000;

pub struct NetworkServerActor {
    listener: TcpListener,
    daq_sender: mpsc::Sender<DaqCommand>,
    session_manager: SessionManager,
    request_id_counter: Arc<AtomicU32>,
    pending_requests: Arc<tokio::sync::RwLock<HashMap<u32, mpsc::Sender<ControlResponse>>>>,
}

impl NetworkServerActor {
    pub async fn new(
        addr: &str,
        daq_sender: mpsc::Sender<DaqCommand>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        info!("Network server listening on {}", addr);

        Ok(Self {
            listener,
            daq_sender,
            session_manager: SessionManager::new(),
            request_id_counter: Arc::new(AtomicU32::new(0)),
            pending_requests: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut heartbeat_interval = interval(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        let mut cleanup_interval = interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                result = self.listener.accept() => {
                    match result {
                        Ok((socket, addr)) => {
                            let daq_sender = self.daq_sender.clone();
                            let session_manager = self.session_manager.clone();
                            let request_id_counter = self.request_id_counter.clone();
                            let pending_requests = self.pending_requests.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_client(
                                    socket,
                                    addr,
                                    daq_sender,
                                    session_manager,
                                    request_id_counter,
                                    pending_requests,
                                )
                                .await
                                {
                                    warn!("Client {} error: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => error!("Accept error: {}", e),
                    }
                }

                _ = heartbeat_interval.tick() => {
                    debug!("Heartbeat tick");
                }

                _ = cleanup_interval.tick() => {
                    self.session_manager.cleanup_expired_sessions().await;
                }
            }
        }
    }

    async fn handle_client(
        mut socket: TcpStream,
        addr: SocketAddr,
        daq_sender: mpsc::Sender<DaqCommand>,
        session_manager: SessionManager,
        request_id_counter: Arc<AtomicU32>,
        pending_requests: Arc<tokio::sync::RwLock<HashMap<u32, mpsc::Sender<ControlResponse>>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Client connected: {}", addr);

        let session_id = Uuid::new_v4().to_string();
        let client_id = format!("client-{}", addr);

        let _session = session_manager
            .create_session(session_id.clone(), client_id.clone(), SESSION_TIMEOUT_SECS)
            .await;

        let mut buf = vec![0u8; 4096];

        loop {
            let read_result = timeout(
                Duration::from_millis(READ_TIMEOUT_MS),
                socket.read(&mut buf),
            )
            .await;

            match read_result {
                Ok(Ok(n)) if n == 0 => {
                    info!("Client {} disconnected", addr);
                    break;
                }
                Ok(Ok(n)) => {
                    let data = &buf[..n];

                    match ControlRequest::decode(data) {
                        Ok(req) => {
                            session_manager.update_heartbeat(&session_id).await;

                            let response = Self::process_request(
                                req,
                                &daq_sender,
                                &request_id_counter,
                                &pending_requests,
                            )
                            .await;

                            let encoded = response.encode();
                            if let Err(e) = socket.write_all(&encoded).await {
                                error!("Failed to write response: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to decode request: {}", e);
                            let error_response = ControlResponse::error(
                                0,
                                format!("Failed to decode request: {}", e),
                            );
                            let encoded = error_response.encode();
                            let _ = socket.write_all(&encoded).await;
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("Read error: {}", e);
                    break;
                }
                Err(_) => {
                    warn!("Read timeout for client {}", addr);

                    if !session_manager
                        .get_session(&session_id)
                        .await
                        .map_or(false, |s| s.is_active())
                    {
                        info!("Session {} timeout", session_id);
                        break;
                    }
                }
            }
        }

        session_manager.remove_session(&session_id).await;
        info!("Client {} session closed", addr);

        Ok(())
    }

    async fn process_request(
        req: ControlRequest,
        daq_sender: &mpsc::Sender<DaqCommand>,
        request_id_counter: &Arc<AtomicU32>,
        pending_requests: &Arc<tokio::sync::RwLock<HashMap<u32, mpsc::Sender<ControlResponse>>>>,
    ) -> ControlResponse {
        let request_id = request_id_counter.fetch_add(1, Ordering::SeqCst);

        match req.request_type {
            RequestType::GetInstruments => {
                Self::handle_get_instruments(req.request_id, daq_sender).await
            }
            RequestType::Heartbeat => Self::handle_heartbeat(req.request_id),
            RequestType::StartRecording => {
                Self::handle_start_recording(req.request_id, req.payload, daq_sender).await
            }
            RequestType::StopRecording => {
                Self::handle_stop_recording(req.request_id, daq_sender).await
            }
            RequestType::SpawnInstrument => {
                Self::handle_spawn_instrument(req.request_id, req.payload, daq_sender).await
            }
            RequestType::StopInstrument => {
                Self::handle_stop_instrument(req.request_id, req.payload, daq_sender).await
            }
            RequestType::SendCommand => {
                Self::handle_send_command(req.request_id, req.payload, daq_sender).await
            }
        }
    }

    async fn handle_get_instruments(
        request_id: u32,
        daq_sender: &mpsc::Sender<DaqCommand>,
    ) -> ControlResponse {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let cmd = DaqCommand::GetInstrumentList { response: tx };

        match daq_sender.send(cmd).await {
            Ok(_) => match timeout(Duration::from_secs(5), rx).await {
                Ok(Ok(instruments)) => {
                    let payload = serde_json::to_vec(&instruments).unwrap_or_default();
                    ControlResponse::new(request_id, ResponseStatus::Success, payload)
                }
                Ok(Err(_)) => {
                    ControlResponse::error(request_id, "Failed to get instruments".to_string())
                }
                Err(_) => ControlResponse::error(request_id, "Request timeout".to_string()),
            },
            Err(e) => ControlResponse::error(request_id, format!("Failed to send command: {}", e)),
        }
    }

    async fn handle_heartbeat(request_id: u32) -> ControlResponse {
        let response = ControlResponse::new(request_id, ResponseStatus::Success, vec![]);
        debug!("Heartbeat acknowledged for request {}", request_id);
        response
    }

    async fn handle_start_recording(
        request_id: u32,
        payload: Vec<u8>,
        daq_sender: &mpsc::Sender<DaqCommand>,
    ) -> ControlResponse {
        // Payload is ignored - configuration is in settings
        let (tx, rx) = tokio::sync::oneshot::channel();
        let cmd = DaqCommand::StartRecording { response: tx };

        match daq_sender.send(cmd).await {
            Ok(_) => match timeout(Duration::from_secs(5), rx).await {
                Ok(Ok(())) => {
                    ControlResponse::new(request_id, ResponseStatus::Success, vec![])
                }
                Ok(Err(_)) => ControlResponse::error(
                    request_id,
                    "Failed to start recording".to_string(),
                ),
                Err(_) => ControlResponse::error(request_id, "Request timeout".to_string()),
            },
            Err(e) => {
                ControlResponse::error(request_id, format!("Failed to send command: {}", e))
            }
        }
    }

    async fn handle_stop_recording(
        request_id: u32,
        daq_sender: &mpsc::Sender<DaqCommand>,
    ) -> ControlResponse {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let cmd = DaqCommand::StopRecording { response: tx };

        match daq_sender.send(cmd).await {
            Ok(_) => match timeout(Duration::from_secs(5), rx).await {
                Ok(Ok(())) => ControlResponse::new(request_id, ResponseStatus::Success, vec![]),
                Ok(Err(_)) => {
                    ControlResponse::error(request_id, "Failed to stop recording".to_string())
                }
                Err(_) => ControlResponse::error(request_id, "Request timeout".to_string()),
            },
            Err(e) => ControlResponse::error(request_id, format!("Failed to send command: {}", e)),
        }
    }

    async fn handle_spawn_instrument(
        request_id: u32,
        payload: Vec<u8>,
        daq_sender: &mpsc::Sender<DaqCommand>,
    ) -> ControlResponse {
        match String::from_utf8(payload) {
            Ok(instrument_id) => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let cmd = DaqCommand::SpawnInstrument {
                    id: instrument_id,
                    response: tx,
                };

                match daq_sender.send(cmd).await {
                    Ok(_) => match timeout(Duration::from_secs(5), rx).await {
                        Ok(Ok(())) => {
                            ControlResponse::new(request_id, ResponseStatus::Success, vec![])
                        }
                        Ok(Err(e)) => ControlResponse::error(
                            request_id,
                            format!("Failed to spawn instrument: {}", e),
                        ),
                        Err(_) => {
                            ControlResponse::error(request_id, "Request timeout".to_string())
                        }
                    },
                    Err(e) => ControlResponse::error(
                        request_id,
                        format!("Failed to send command: {}", e),
                    ),
                }
            }
            Err(e) => ControlResponse::error(request_id, format!("Invalid payload: {}", e)),
        }
    }

    async fn handle_stop_instrument(
        request_id: u32,
        payload: Vec<u8>,
        daq_sender: &mpsc::Sender<DaqCommand>,
    ) -> ControlResponse {
        match String::from_utf8(payload) {
            Ok(instrument_id) => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let cmd = DaqCommand::StopInstrument {
                    id: instrument_id,
                    response: tx,
                };

                match daq_sender.send(cmd).await {
                    Ok(_) => match timeout(Duration::from_secs(5), rx).await {
                        Ok(()) => {
                            ControlResponse::new(request_id, ResponseStatus::Success, vec![])
                        }
                        Err(_) => ControlResponse::error(request_id, "Request timeout".to_string()),
                    },
                    Err(e) => {
                        ControlResponse::error(request_id, format!("Failed to send command: {}", e))
                    }
                }
            }
            Err(e) => ControlResponse::error(request_id, format!("Invalid payload: {}", e)),
        }
    }

    /// Parse JSON payload into InstrumentCommand enum
    fn parse_instrument_command(cmd_json: &serde_json::Value) -> Result<InstrumentCommand, String> {
        let command_type = cmd_json
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'type' field in command".to_string())?;

        match command_type {
            "set_parameter" => {
                let key = cmd_json
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'key' field".to_string())?
                    .to_string();
                
                let value_json = cmd_json
                    .get("value")
                    .ok_or_else(|| "Missing 'value' field".to_string())?;
                
                let value = ParameterValue::from_json(value_json)
                    .map_err(|e| format!("Invalid parameter value: {}", e))?;
                
                Ok(InstrumentCommand::SetParameter(key, value))
            }
            "query_parameter" => {
                let key = cmd_json
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'key' field".to_string())?
                    .to_string();
                
                Ok(InstrumentCommand::QueryParameter(key))
            }
            "execute" => {
                let cmd = cmd_json
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'command' field".to_string())?
                    .to_string();
                
                let args = cmd_json
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                
                Ok(InstrumentCommand::Execute(cmd, args))
            }
            _ => Err(format!("Unknown command type: {}", command_type)),
        }
    }

    async fn handle_send_command(
        request_id: u32,
        payload: Vec<u8>,
        daq_sender: &mpsc::Sender<DaqCommand>,
    ) -> ControlResponse {
        match String::from_utf8(payload) {
            Ok(command_json) => match serde_json::from_str::<serde_json::Value>(&command_json) {
                Ok(cmd_json) => {
                    let instrument_id = cmd_json
                        .get("instrument_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    match Self::parse_instrument_command(&cmd_json) {
                        Ok(instrument_command) => {
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            let cmd = DaqCommand::SendInstrumentCommand {
                                id: instrument_id,
                                command: instrument_command,
                                response: tx,
                            };

                            match daq_sender.send(cmd).await {
                                Ok(_) => match timeout(Duration::from_secs(5), rx).await {
                                    Ok(Ok(())) => {
                                        ControlResponse::new(request_id, ResponseStatus::Success, vec![])
                                    }
                                    Ok(Err(_)) => ControlResponse::error(
                                        request_id,
                                        "Failed to send command".to_string(),
                                    ),
                                    Err(_) => {
                                        ControlResponse::error(request_id, "Request timeout".to_string())
                                    }
                                },
                                Err(e) => ControlResponse::error(
                                    request_id,
                                    format!("Failed to send command: {}", e),
                                ),
                            }
                        }
                        Err(e) => ControlResponse::error(request_id, format!("Invalid command: {}", e)),
                    }
                }
                Err(e) => {
                    ControlResponse::error(request_id, format!("Invalid JSON command: {}", e))
                }
            },
            Err(e) => ControlResponse::error(request_id, format!("Invalid payload: {}", e)),
        }
    }
}