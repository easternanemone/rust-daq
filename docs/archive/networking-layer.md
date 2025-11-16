# Networking Layer Design: WebSocket-Based Instrument Sharing

**Issue**: bd-63 (Phase 3A)
**Status**: Design Complete
**Author**: Claude Code + Multi-Model Consensus (Gemini 2.5 Pro)
**Date**: 2025-10-19
**Target**: rust-daq Phase 3A - DynExp Networking Parity

## Executive Summary

This document specifies the WebSocket-based networking layer for rust-daq, enabling remote instrument sharing across DAQ instances. The design achieves **<10ms loopback latency** through a dual-socket architecture with zero-copy serialization, following proven patterns from HiSLIP (Test & Measurement industry standard) and modern high-performance systems.

**Key Design Decisions** (validated via multi-model consensus):
- **Transport**: WebSocket (not gRPC) - simpler, proven <100ms latency, excellent Rust async support
- **Architecture**: Dual WebSocket (HiSLIP pattern) - separate control/data channels prevent head-of-line blocking
- **Serialization**: FlatBuffers (not Protobuf) - zero-copy aligns with `Arc<Measurement>` architecture
- **Security**: JWT authentication via `Sec-WebSocket-Protocol` header + optional TLS
- **Heartbeat**: 2s interval / 6s timeout - rapid partition detection for DAQ reliability

## 1. Requirements & Constraints

### 1.1 Functional Requirements

- **FR1**: Remote instrument access across network boundaries
- **FR2**: Transparent remoting - RemoteInstrument implements Instrument trait identically to local instruments
- **FR3**: Support all Measurement variants (Scalar: 8 bytes, Spectrum: ~KB, Image: MB+)
- **FR4**: Network partition recovery with eventual consistency
- **FR5**: Optional TLS encryption (feature flag: `networking_tls`)
- **FR6**: JWT-based authentication for access control
- **FR7**: Integration test: 2 DAQ instances sharing 1 mock instrument

### 1.2 Performance Requirements

- **PR1**: <10ms loopback command latency (99th percentile)
- **PR2**: Support streaming 1024 measurements/sec (existing broadcast channel capacity)
- **PR3**: Zero-copy data transmission where possible
- **PR4**: Non-blocking async I/O (no tokio thread starvation)

### 1.3 Constraints

- **C1**: Must integrate with existing Tokio runtime and async Instrument trait
- **C2**: Feature flag isolation: `networking` compiles independently
- **C3**: Leverage existing `Arc<Measurement>` zero-copy architecture (Phase 2)
- **C4**: Graceful shutdown with 5s timeout (existing pattern from bd-20)
- **C5**: Error handling via `DaqError` enum (existing error infrastructure)

## 2. Architecture Overview

### 2.1 System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        DAQ Instance A (Server)                   │
│                                                                  │
│  ┌────────────────┐         ┌──────────────────────────────┐   │
│  │  LocalInstrument│────────▶│    InstrumentServer         │   │
│  │  (MockInstrument│         │  - WebSocket Listener        │   │
│  │   ESP300, etc.) │         │  - Authentication Handler    │   │
│  └────────────────┘         │  - Connection Manager        │   │
│         │                    │  - Message Router            │   │
│         │ Arc<Measurement>   └──────────────────────────────┘   │
│         ▼                              │ │                      │
│  ┌────────────────┐                   │ │ Dual WebSocket      │
│  │ Broadcast Chan │                   │ │ (Control + Data)     │
│  │  (capacity:    │                   │ │                      │
│  │    1024)       │                   │ │                      │
│  └────────────────┘                   │ │                      │
└─────────────────────────────────────────┼─┼──────────────────────┘
                                          │ │
                                          │ │ Network (TLS optional)
                                          │ │
┌─────────────────────────────────────────┼─┼──────────────────────┐
│                        DAQ Instance B (Client)                   │
│                                          │ │                      │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                  RemoteInstrument                           │ │
│  │  implements Instrument trait                                │ │
│  │  ┌──────────────────┐  ┌────────────────────────┐          │ │
│  │  │ Control Client   │  │    Data Client         │          │ │
│  │  │ - Send commands  │  │  - Receive measurements│          │ │
│  │  │ - Receive acks   │  │  - FlatBuffers decode  │          │ │
│  │  │ - Heartbeats     │  │  - Arc<Measurement>    │          │ │
│  │  └──────────────────┘  └────────────────────────┘          │ │
│  └────────────────────────────────────────────────────────────┘ │
│         │                                                         │
│         │ Arc<Measurement>                                        │
│         ▼                                                         │
│  ┌────────────────┐                                              │
│  │ Local Broadcast│                                              │
│  │    Channel     │───▶ GUI / Storage / Processors               │
│  └────────────────┘                                              │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Dual WebSocket Pattern (HiSLIP-inspired)

**Why Dual Sockets?**
- **Control Channel**: Low-latency commands (connect, disconnect, set_parameter, get_status)
- **Data Channel**: High-throughput streaming (measurements, never blocks control)
- **Prevents head-of-line blocking**: Large image data (multi-MB) won't delay commands

**Analogy to HiSLIP** (LXI Test & Measurement standard):
- HiSLIP separates synchronous (commands) and asynchronous (data) TCP connections
- rust-daq mirrors this with two WebSocket connections sharing authentication state

### 2.3 Zero-Copy Data Flow

```
Server Side:
LocalInstrument → Arc<Measurement> → FlatBuffers serialize (in-place) → WebSocket send

Client Side:
WebSocket recv → FlatBuffers buffer (zero-copy read) → Arc<Measurement> → Broadcast Channel
```

**Critical**: No Protobuf intermediate copy. FlatBuffers allows reading directly from network buffer.

## 3. WebSocket Protocol Specification

### 3.1 Connection Lifecycle

```
State Machine for RemoteInstrument:

[Disconnected]
    ↓ connect()
[Authenticating] ──(JWT validation fails)──▶ [Disconnected]
    ↓ (JWT valid)
[Handshaking] ──(instrument not found)──▶ [Disconnected]
    ↓ (success)
[Connected] ──(network partition)──▶ [Reconnecting] ──(timeout)──▶ [Disconnected]
    ↓                                        ↑ (retry loop)
    │◀───────────────────────────────────────┘
    ↓ disconnect()
[Disconnecting] ──(cleanup)──▶ [Disconnected]
```

### 3.2 Message Types (FlatBuffers Schema)

```flatbuffers
// src/network/protocol.fbs

namespace rust_daq.network;

// Message envelope for control channel
table ControlMessage {
  id: ulong;                      // Monotonic request ID
  type: ControlMessageType;
  payload: [ubyte] (nested_flatbuffer: "ControlPayload");
}

enum ControlMessageType : byte {
  ConnectRequest = 0,
  ConnectResponse = 1,
  CommandRequest = 2,
  CommandResponse = 3,
  Heartbeat = 4,
  HeartbeatAck = 5,
  Disconnect = 6,
  Error = 7
}

// Control channel payloads
union ControlPayload {
  ConnectRequest,
  ConnectResponse,
  CommandRequest,
  CommandResponse,
  Heartbeat,
  HeartbeatAck,
  ErrorResponse
}

table ConnectRequest {
  instrument_id: string;           // Which instrument to access
  client_id: string;               // UUID for client identification
  protocol_version: ushort;        // Currently 1
}

table ConnectResponse {
  success: bool;
  session_id: string;              // Server-assigned session ID
  error_message: string;           // If success=false
  instrument_metadata: InstrumentMetadata;
}

table InstrumentMetadata {
  name: string;
  channels: [string];              // Available channel IDs
  supported_commands: [string];    // e.g., ["set_wavelength", "get_power"]
}

table CommandRequest {
  command: InstrumentCommand;
}

// Maps to rust-daq InstrumentCommand enum
union InstrumentCommand {
  SetParameter,
  GetParameter,
  Shutdown
}

table SetParameter {
  name: string;
  value: string;                   // JSON-serialized for flexibility
}

table GetParameter {
  name: string;
}

table CommandResponse {
  success: bool;
  result: string;                  // JSON-serialized result
  error_message: string;
}

table Heartbeat {
  timestamp_ns: ulong;             // Client's monotonic clock (ns)
}

table HeartbeatAck {
  client_timestamp_ns: ulong;      // Echo client's timestamp
  server_timestamp_ns: ulong;      // Server's timestamp for clock sync
}

table ErrorResponse {
  code: ErrorCode;
  message: string;
  details: string;
}

enum ErrorCode : ushort {
  InstrumentNotFound = 1000,
  InstrumentBusy = 1001,
  InstrumentDisconnected = 1002,
  AuthenticationFailed = 1003,
  InvalidCommand = 1004,
  CommandTimeout = 1005,
  ProtocolError = 1006,
  InternalServerError = 1007
}

// Data channel message
table DataMessage {
  session_id: string;
  sequence: ulong;                 // Monotonic sequence for ordering
  measurement: Measurement;
}

// Maps to daq-core::Measurement enum
union MeasurementData {
  ScalarMeasurement,
  SpectrumMeasurement,
  ImageMeasurement
}

table Measurement {
  timestamp_ns: long;              // UTC timestamp (ns since epoch)
  channel: string;
  data: MeasurementData;
}

table ScalarMeasurement {
  value: double;
  unit: string;
  metadata: string;                // JSON-serialized (optional)
}

table SpectrumMeasurement {
  wavelengths: [double];           // Or frequencies
  intensities: [double];
  unit: string;
  metadata: string;
}

table ImageMeasurement {
  width: uint;
  height: uint;
  pixels: [ubyte];                 // Flattened row-major, zero-copy buffer
  pixel_format: PixelFormat;
  metadata: string;
}

enum PixelFormat : byte {
  Grayscale8 = 0,
  Grayscale16 = 1,
  RGB8 = 2,
  RGB16 = 3
}

root_type ControlMessage;
```

### 3.3 Connection Handshake Sequence

```
Client                                Server
  │                                     │
  │  WebSocket Upgrade (Control)       │
  │  Sec-WebSocket-Protocol: jwt.<token>
  ├────────────────────────────────────▶│
  │                                     │ Validate JWT
  │                                     │ Check token signature
  │                                     │ Extract claims (user_id, exp)
  │  101 Switching Protocols           │
  │◀────────────────────────────────────┤
  │                                     │
  │  WebSocket Upgrade (Data)          │
  │  Sec-WebSocket-Protocol: jwt.<token>
  ├────────────────────────────────────▶│
  │                                     │ Validate same JWT
  │  101 Switching Protocols           │
  │◀────────────────────────────────────┤
  │                                     │
  │  ConnectRequest(instrument_id)     │
  ├────────────────────────────────────▶│
  │                                     │ Check instrument available
  │                                     │ Create session
  │                                     │ Subscribe to broadcast channel
  │  ConnectResponse(session_id)       │
  │◀────────────────────────────────────┤
  │                                     │
  │  [Connected - start heartbeats]    │
  │                                     │
  │  Heartbeat (every 2s)              │
  ├────────────────────────────────────▶│
  │  HeartbeatAck                       │
  │◀────────────────────────────────────┤
  │                                     │
  │  [Data streaming on data channel]  │
  │  DataMessage(Measurement)           │
  │◀────────────────────────────────────┤
  │  DataMessage(Measurement)           │
  │◀────────────────────────────────────┤
  │  ...                                │
```

### 3.4 Authentication Flow

**JWT Structure**:
```json
{
  "header": {
    "alg": "HS256",
    "typ": "JWT"
  },
  "payload": {
    "sub": "user123",               // User identifier
    "exp": 1729368000,              // Expiration (Unix timestamp)
    "iat": 1729364400,              // Issued at
    "roles": ["operator", "admin"], // RBAC roles
    "instruments": ["*"]            // Allowed instruments ("*" = all)
  }
}
```

**Validation Steps** (server-side):
1. Extract JWT from `Sec-WebSocket-Protocol` header
2. Verify signature using shared secret (HS256) or public key (RS256)
3. Check expiration (`exp` claim)
4. Validate issuer (`iss` claim) if configured
5. Extract user identity and permissions
6. Return 401 Unauthorized if validation fails

**Security Notes**:
- Use HS256 for single-server deployments (shared secret)
- Use RS256 for distributed deployments (public/private key pair)
- Refresh tokens not needed for WebSocket (reconnect on expiration)
- TLS required for production (JWT tokens are bearer tokens)

## 4. Component Design

### 4.1 InstrumentServer

**File**: `src/network/server.rs`

**Responsibilities**:
- WebSocket server listening on configurable port (default: 8080)
- JWT authentication validation
- Session management (map session_id → client connection)
- Message routing between clients and local instruments
- Heartbeat monitoring

**Key Structures**:

```rust
pub struct InstrumentServer {
    /// Server configuration
    config: ServerConfig,

    /// Map instrument_id → Arc<Mutex<dyn Instrument>>
    instruments: Arc<RwLock<HashMap<String, InstrumentHandle>>>,

    /// Active client sessions
    sessions: Arc<RwLock<HashMap<String, ClientSession>>>,

    /// JWT secret for validation
    jwt_secret: Arc<Vec<u8>>,

    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
}

struct InstrumentHandle {
    /// Reference to local instrument
    instrument: Arc<Mutex<dyn Instrument>>,

    /// Broadcast receiver for data streaming
    data_rx: broadcast::Receiver<Arc<Measurement>>,

    /// Command sender for control
    cmd_tx: mpsc::Sender<InstrumentCommand>,
}

struct ClientSession {
    session_id: String,
    client_id: String,
    instrument_id: String,
    user_id: String,

    /// Control channel WebSocket
    control_tx: mpsc::Sender<ControlMessage>,

    /// Data channel WebSocket
    data_tx: mpsc::Sender<DataMessage>,

    /// Last heartbeat timestamp
    last_heartbeat: Arc<Mutex<Instant>>,

    /// Session start time
    created_at: Instant,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,        // "0.0.0.0:8080"
    pub tls_enabled: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub jwt_secret: String,
    pub heartbeat_interval_secs: u64,   // 2
    pub heartbeat_timeout_secs: u64,    // 6
    pub max_sessions_per_instrument: usize, // 10 (configurable)
}
```

**Key Methods**:

```rust
impl InstrumentServer {
    pub async fn new(config: ServerConfig) -> Result<Self>;

    pub async fn register_instrument(
        &self,
        id: String,
        instrument: Arc<Mutex<dyn Instrument>>
    ) -> Result<()>;

    pub async fn run(&self) -> Result<()>;

    async fn handle_client_connection(
        &self,
        ws_stream: WebSocketStream<TcpStream>,
        remote_addr: SocketAddr,
    ) -> Result<()>;

    async fn authenticate_connection(
        &self,
        headers: &HeaderMap
    ) -> Result<JwtClaims>;

    async fn handle_control_message(
        &self,
        session: &ClientSession,
        msg: ControlMessage,
    ) -> Result<()>;

    async fn stream_data_to_client(
        &self,
        session: ClientSession,
    ) -> Result<()>;

    async fn monitor_heartbeats(&self) -> Result<()>;
}
```

### 4.2 RemoteInstrument

**File**: `src/network/client.rs`

**Responsibilities**:
- Implements `Instrument` trait identically to local instruments
- Manages dual WebSocket connections (control + data)
- Handles reconnection logic on network partition
- Translates local Instrument trait calls to network messages

**Key Structures**:

```rust
pub struct RemoteInstrument {
    /// Unique identifier for logging
    id: String,

    /// Server endpoint
    server_url: String,

    /// JWT token for authentication
    jwt_token: String,

    /// Control channel state
    control_state: Arc<Mutex<ControlChannelState>>,

    /// Data channel receiver
    data_rx: Arc<Mutex<Option<broadcast::Receiver<Arc<Measurement>>>>>,

    /// Local broadcast sender (for transparent integration)
    local_broadcast_tx: Option<broadcast::Sender<Arc<Measurement>>>,

    /// Configuration
    config: RemoteInstrumentConfig,

    /// Reconnection state
    reconnect_state: Arc<Mutex<ReconnectState>>,
}

struct ControlChannelState {
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    session_id: Option<String>,
    last_heartbeat_sent: Instant,
    next_request_id: AtomicU64,
    pending_requests: HashMap<u64, oneshot::Sender<CommandResponse>>,
}

struct ReconnectState {
    attempt_count: usize,
    last_attempt: Instant,
    backoff_delay: Duration,
}

#[derive(Deserialize)]
pub struct RemoteInstrumentConfig {
    pub instrument_id: String,
    pub server_url: String,
    pub jwt_token: String,
    pub reconnect_enabled: bool,
    pub reconnect_max_attempts: usize,     // 10
    pub reconnect_initial_delay_ms: u64,   // 100
    pub reconnect_max_delay_ms: u64,       // 30000
}
```

**Key Methods** (Instrument trait implementation):

```rust
#[async_trait]
impl Instrument for RemoteInstrument {
    fn name(&self) -> String {
        self.id.clone()
    }

    async fn connect(&mut self, settings: &Arc<Settings>) -> Result<()> {
        // 1. Establish control WebSocket connection
        let control_ws = self.connect_websocket("control").await?;

        // 2. Establish data WebSocket connection
        let data_ws = self.connect_websocket("data").await?;

        // 3. Send ConnectRequest on control channel
        let connect_req = ConnectRequest {
            instrument_id: self.config.instrument_id.clone(),
            client_id: Uuid::new_v4().to_string(),
            protocol_version: 1,
        };

        let response = self.send_control_message(connect_req).await?;

        // 4. Store session_id
        self.control_state.lock().await.session_id = Some(response.session_id);

        // 5. Start heartbeat task
        self.start_heartbeat_task().await;

        // 6. Start data streaming task
        self.start_data_streaming_task(data_ws).await;

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // Send Disconnect message
        let disconnect_msg = ControlMessage {
            id: self.next_request_id(),
            type: ControlMessageType::Disconnect,
            payload: vec![],
        };

        self.send_control_message_no_response(disconnect_msg).await?;

        // Close WebSocket connections
        self.control_state.lock().await.ws_stream = None;
        self.data_rx.lock().await = None;

        Ok(())
    }

    async fn data_stream(&mut self) -> Result<broadcast::Receiver<Arc<Measurement>>> {
        // Return receiver for local broadcast channel
        // Data streaming task forwards network messages to this channel
        self.local_broadcast_tx
            .as_ref()
            .map(|tx| tx.subscribe())
            .ok_or_else(|| anyhow!("Not connected"))
    }

    async fn handle_command(&mut self, cmd: InstrumentCommand) -> Result<()> {
        // Translate InstrumentCommand to CommandRequest
        let cmd_req = CommandRequest {
            command: self.translate_command(cmd)?,
        };

        // Send via control channel
        let response = self.send_control_message(cmd_req).await?;

        if !response.success {
            return Err(anyhow!("Command failed: {}", response.error_message));
        }

        Ok(())
    }
}

// Internal methods
impl RemoteInstrument {
    async fn connect_websocket(&self, channel_type: &str) -> Result<WebSocketStream> {
        let url = format!("{}/{}", self.server_url, channel_type);

        let request = Request::builder()
            .uri(&url)
            .header("Sec-WebSocket-Protocol", format!("jwt.{}", self.jwt_token))
            .body(())?;

        let (ws_stream, _response) = tokio_tungstenite::connect_async(request).await?;

        Ok(ws_stream)
    }

    async fn send_control_message<T>(&self, msg: T) -> Result<CommandResponse>
    where
        T: Into<ControlMessage>
    {
        let request_id = self.next_request_id();
        let (response_tx, response_rx) = oneshot::channel();

        // Register pending request
        self.control_state.lock().await
            .pending_requests
            .insert(request_id, response_tx);

        // Serialize with FlatBuffers
        let mut builder = FlatBufferBuilder::new();
        let msg: ControlMessage = msg.into();
        let fb_msg = msg.to_flatbuffer(&mut builder);
        builder.finish(fb_msg, None);
        let bytes = builder.finished_data();

        // Send via WebSocket
        let ws = self.control_state.lock().await.ws_stream.as_mut()
            .ok_or_else(|| anyhow!("Not connected"))?;
        ws.send(Message::Binary(bytes.to_vec())).await?;

        // Wait for response (with timeout)
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            response_rx
        ).await??;

        Ok(response)
    }

    async fn start_heartbeat_task(&self) {
        let control_state = Arc::clone(&self.control_state);
        let interval = Duration::from_secs(2);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                let heartbeat = Heartbeat {
                    timestamp_ns: Instant::now().elapsed().as_nanos() as u64,
                };

                // Send heartbeat (best-effort, no response required)
                if let Err(e) = send_heartbeat(&control_state, heartbeat).await {
                    log::warn!("Heartbeat send failed: {}", e);
                    // Trigger reconnection logic
                    break;
                }
            }
        });
    }

    async fn start_data_streaming_task(&self, mut data_ws: WebSocketStream) {
        let local_broadcast_tx = self.local_broadcast_tx.clone();

        tokio::spawn(async move {
            loop {
                match data_ws.next().await {
                    Some(Ok(Message::Binary(bytes))) => {
                        // Zero-copy FlatBuffers deserialization
                        let data_msg = flatbuffers::root::<DataMessage>(&bytes)?;

                        // Convert to Arc<Measurement>
                        let measurement = Arc::new(data_msg.measurement().into());

                        // Forward to local broadcast channel
                        if let Some(tx) = local_broadcast_tx.as_ref() {
                            let _ = tx.send(measurement);
                        }
                    }
                    Some(Err(e)) => {
                        log::error!("Data channel error: {}", e);
                        // Trigger reconnection
                        break;
                    }
                    None => {
                        log::info!("Data channel closed");
                        break;
                    }
                }
            }
        });
    }

    async fn handle_network_partition(&self) -> Result<()> {
        let mut reconnect_state = self.reconnect_state.lock().await;

        if !self.config.reconnect_enabled {
            return Err(anyhow!("Reconnection disabled"));
        }

        if reconnect_state.attempt_count >= self.config.reconnect_max_attempts {
            return Err(anyhow!("Max reconnection attempts exceeded"));
        }

        // Exponential backoff
        let delay = std::cmp::min(
            Duration::from_millis(self.config.reconnect_initial_delay_ms)
                * 2_u32.pow(reconnect_state.attempt_count as u32),
            Duration::from_millis(self.config.reconnect_max_delay_ms)
        );

        tokio::time::sleep(delay).await;

        reconnect_state.attempt_count += 1;
        reconnect_state.last_attempt = Instant::now();

        // Attempt reconnection
        // Note: connect() will reset attempt_count on success
        self.connect(&Arc::new(Settings::default())).await?;

        Ok(())
    }
}
```

### 4.3 FlatBuffers Integration

**Build Integration** (`build.rs`):

```rust
// build.rs
fn main() {
    #[cfg(feature = "networking")]
    {
        // Compile FlatBuffers schema
        flatc::run(flatc::Args {
            inputs: &["src/network/protocol.fbs"],
            out_dir: "src/network/generated/",
            lang: "rust",
            ..Default::default()
        });
    }
}
```

**Conversion Helpers** (`src/network/conversions.rs`):

```rust
// Convert daq-core::Measurement to FlatBuffers DataMessage
impl From<Arc<Measurement>> for DataMessage {
    fn from(m: Arc<Measurement>) -> Self {
        match m.as_ref() {
            Measurement::Scalar(dp) => {
                // Build ScalarMeasurement
                // ...
            }
            Measurement::Spectrum(sd) => {
                // Build SpectrumMeasurement
                // Zero-copy: store wavelengths/intensities as slices
                // ...
            }
            Measurement::Image(id) => {
                // Build ImageMeasurement
                // Zero-copy: pixels buffer directly references Arc<ImageData>
                // ...
            }
        }
    }
}

// Convert FlatBuffers DataMessage to Arc<Measurement>
impl From<&DataMessage> for Arc<Measurement> {
    fn from(msg: &DataMessage) -> Self {
        match msg.measurement().data_type() {
            MeasurementData::ScalarMeasurement => {
                let scalar = msg.measurement().data_as_scalar_measurement().unwrap();
                Arc::new(Measurement::Scalar(daq_core::DataPoint {
                    timestamp: /* convert */,
                    channel: scalar.channel().to_string(),
                    value: scalar.value(),
                    unit: scalar.unit().to_string(),
                    metadata: /* parse JSON */,
                }))
            }
            // Similar for Spectrum and Image
            // ...
        }
    }
}
```

## 5. Network Partition Recovery

### 5.1 Detection Mechanisms

**Heartbeat Monitoring**:
- Client sends `Heartbeat` every 2s on control channel
- Server responds with `HeartbeatAck` echoing client timestamp
- Client marks server unreachable after 3 missed acks (6s timeout)
- Server marks client unreachable after 3 missed heartbeats (6s timeout)

**WebSocket Connection State**:
- WebSocket close frames trigger immediate partition detection
- TCP connection errors (ECONNRESET, ETIMEDOUT) trigger partition
- TLS handshake failures trigger authentication re-check

### 5.2 Recovery Strategy

**Client-Side Reconnection**:
1. Enter `Reconnecting` state
2. Exponential backoff: 100ms, 200ms, 400ms, ..., max 30s
3. Re-establish control WebSocket (re-authenticate with same JWT)
4. Re-establish data WebSocket
5. Send `ConnectRequest` with same `client_id` (server recognizes session)
6. Resume from last received `sequence` number (server resends missed data)
7. Reset heartbeat timer
8. Return to `Connected` state

**Server-Side Handling**:
- Keep session state for 60s after client disconnect (grace period)
- Buffer recent measurements (last 1024) for resend on reconnect
- If client reconnects within 60s: resume session, resend buffered data
- If 60s expires: clean up session, require full re-connect

**Graceful Degradation**:
- Local DAQ instance continues acquiring data during partition
- Measurements are not dropped (broadcast channel buffering)
- When partition heals, client receives buffered + new measurements

### 5.3 Conflict Resolution

For instrument control scenarios:
- **Read operations**: Always succeed (query server's current state)
- **Write operations** during partition: Fail immediately (cannot reach server)
- **No optimistic writes**: Client must be connected to send commands

For measurement data:
- **Timestamps provide ordering**: No conflicts (append-only stream)
- **Sequence numbers**: Detect gaps, request resend from server

## 6. Security Design

### 6.1 Transport Layer Security (TLS)

**Configuration** (optional, feature flag: `networking_tls`):

```toml
[network.server]
tls_enabled = true
tls_cert_path = "/etc/rust-daq/certs/server.crt"
tls_key_path = "/etc/rust-daq/certs/server.key"
```

**Implementation**:
- Use `tokio-native-tls` or `rustls` for async TLS
- Support TLS 1.2 and TLS 1.3
- Recommended cipher suites: TLS_AES_256_GCM_SHA384, TLS_CHACHA20_POLY1305_SHA256
- Certificate validation: Server presents certificate, client validates against system trust store
- Optional: Mutual TLS (client certificates) for high-security environments

### 6.2 Authentication & Authorization

**JWT Claims** (RBAC):

```rust
#[derive(Deserialize)]
struct JwtClaims {
    sub: String,              // User ID
    exp: i64,                 // Expiration timestamp
    iat: i64,                 // Issued at
    roles: Vec<String>,       // ["operator", "admin", "viewer"]
    instruments: Vec<String>, // ["*"] or ["mock", "esp300"]
}
```

**Authorization Rules**:
- **viewer**: Can connect and stream data only (no commands)
- **operator**: Can connect, stream, send parameter changes
- **admin**: Full access (connect, stream, all commands including Shutdown)

**Implementation** (`src/network/auth.rs`):

```rust
pub fn validate_jwt(token: &str, secret: &[u8]) -> Result<JwtClaims> {
    // Use jsonwebtoken crate
    let validation = jsonwebtoken::Validation::new(Algorithm::HS256);
    let token_data = jsonwebtoken::decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &validation
    )?;

    Ok(token_data.claims)
}

pub fn authorize_instrument_access(
    claims: &JwtClaims,
    instrument_id: &str
) -> Result<()> {
    if claims.instruments.contains(&"*".to_string()) {
        return Ok(());
    }

    if claims.instruments.contains(&instrument_id.to_string()) {
        return Ok(());
    }

    Err(anyhow!("Unauthorized access to instrument: {}", instrument_id))
}

pub fn authorize_command(
    claims: &JwtClaims,
    cmd: &InstrumentCommand
) -> Result<()> {
    match cmd {
        InstrumentCommand::Shutdown => {
            if !claims.roles.contains(&"admin".to_string()) {
                return Err(anyhow!("Shutdown requires admin role"));
            }
        }
        InstrumentCommand::SetParameter { .. } => {
            if !claims.roles.contains(&"operator".to_string())
                && !claims.roles.contains(&"admin".to_string()) {
                return Err(anyhow!("SetParameter requires operator or admin role"));
            }
        }
        _ => {} // GetParameter, etc. allowed for all authenticated users
    }

    Ok(())
}
```

### 6.3 Attack Surface Mitigation

**Rate Limiting**:
- Max 100 commands/sec per client (prevent DoS)
- Max 10 concurrent sessions per instrument (prevent resource exhaustion)

**Input Validation**:
- FlatBuffers schema validation (malformed messages rejected)
- Instrument ID whitelist (only registered instruments accessible)
- Command parameter validation (instrument-specific)

**Audit Logging**:
- Log all connection attempts (success/failure)
- Log all commands with user_id, instrument_id, timestamp
- Log authentication failures for intrusion detection

## 7. Performance Optimization

### 7.1 Latency Budget Breakdown (<10ms target)

```
Loopback Command Round-Trip:

Client:
  - Serialize CommandRequest (FlatBuffers)     : 0.1 ms
  - WebSocket send (control channel)          : 0.2 ms

Network (Loopback):
  - Loopback interface latency                : ~0.1 ms

Server:
  - WebSocket receive                         : 0.2 ms
  - Deserialize CommandRequest (FlatBuffers)  : 0.1 ms
  - Instrument command handler                : 2.0 ms (local call)
  - Serialize CommandResponse (FlatBuffers)   : 0.1 ms
  - WebSocket send                            : 0.2 ms

Network (Loopback):
  - Return path                               : ~0.1 ms

Client:
  - WebSocket receive                         : 0.2 ms
  - Deserialize CommandResponse (FlatBuffers) : 0.1 ms

TOTAL:                                         ~3.4 ms (well under 10ms)
```

**Margin for Error**: 6.6ms available for:
- Network jitter
- CPU scheduling delays
- Tokio task queue latency
- GC pauses (N/A in Rust, but worth mentioning for completeness)

### 7.2 Zero-Copy Optimization

**FlatBuffers Advantages**:
- **Reading**: No deserialization overhead. Access data directly from buffer via offsets.
- **Writing**: Build buffer in-place, send buffer directly to WebSocket (no intermediate copy).

**Measurement Data Flow** (Image example):

```rust
// Server side (ZERO intermediate copies)
let measurement: Arc<Measurement> = /* from broadcast channel */;
let image_data = match measurement.as_ref() {
    Measurement::Image(id) => id,
    _ => unreachable!(),
};

// FlatBuffers builder references original pixel data
let mut builder = FlatBufferBuilder::new();
let pixels = builder.create_vector(&image_data.pixels); // Slice reference
let img_fb = ImageMeasurement::create(&mut builder, &ImageMeasurementArgs {
    width: image_data.width,
    height: image_data.height,
    pixels: Some(pixels),
    // ...
});
builder.finish(img_fb, None);

// Send buffer directly (no copy)
ws.send(Message::Binary(builder.finished_data().to_vec())).await?;
// ^^^^^^^^^^^^^^^^^^^^^^^ Only this creates final copy for network I/O
```

### 7.3 Tokio Runtime Tuning

**Configuration** (tokio runtime):
```rust
tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)              // Tune based on CPU cores
    .max_blocking_threads(8)        // For blocking I/O (TLS handshake)
    .thread_name("rust-daq-net")
    .build()?;
```

**Async Task Priorities**:
- Control channel: Higher priority (urgent commands)
- Data channel: Lower priority (buffered streaming)
- Use `tokio::select!` with bias for control messages

## 8. Configuration & Feature Flags

### 8.1 Feature Flag: `networking`

**Cargo.toml**:
```toml
[features]
default = ["storage_csv", "instrument_serial"]
networking = ["dep:tokio-tungstenite", "dep:flatbuffers", "dep:jsonwebtoken"]
networking_tls = ["networking", "dep:tokio-native-tls"]
full = ["networking", "networking_tls", "storage_hdf5", /* ... */]

[dependencies]
tokio-tungstenite = { version = "0.21", optional = true }
flatbuffers = { version = "23.5", optional = true }
jsonwebtoken = { version = "9.2", optional = true }
tokio-native-tls = { version = "0.3", optional = true }
```

**Build Verification**:
```bash
# Test independent compilation
cargo check --no-default-features --features networking

# Test with TLS
cargo check --no-default-features --features networking_tls

# Test without networking (ensure no compile errors)
cargo check --no-default-features --features storage_csv
```

### 8.2 TOML Configuration

**Server Configuration** (`config/network_server.toml`):

```toml
[network.server]
enabled = true
bind_address = "0.0.0.0:8080"
tls_enabled = false
# tls_cert_path = "/etc/rust-daq/certs/server.crt"
# tls_key_path = "/etc/rust-daq/certs/server.key"
jwt_secret = "your-256-bit-secret-here"  # Change in production!
heartbeat_interval_secs = 2
heartbeat_timeout_secs = 6
max_sessions_per_instrument = 10

# Instruments to expose remotely
[[network.server.instruments]]
id = "mock"
enabled = true

[[network.server.instruments]]
id = "esp300"
enabled = true
```

**Client Configuration** (`config/network_client.toml`):

```toml
[instruments.remote_mock]
type = "remote"
server_url = "ws://192.168.1.100:8080"
instrument_id = "mock"
jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
reconnect_enabled = true
reconnect_max_attempts = 10
reconnect_initial_delay_ms = 100
reconnect_max_delay_ms = 30000
```

## 9. Integration Testing

### 9.1 Test Scenarios

**Test 1: Basic Remote Access**
```rust
#[tokio::test]
async fn test_remote_instrument_basic_access() {
    // Setup: Start server with MockInstrument
    let server = InstrumentServer::new(/* config */).await?;
    server.register_instrument("mock", Arc::new(Mutex::new(MockInstrument::new()))).await?;
    tokio::spawn(async move { server.run().await });

    // Setup: Create RemoteInstrument client
    let mut remote = RemoteInstrument::new(/* config */);
    remote.connect(&settings).await?;

    // Verify: Data streaming works
    let mut data_rx = remote.data_stream().await?;
    let measurement = tokio::time::timeout(
        Duration::from_secs(2),
        data_rx.recv()
    ).await??;

    assert!(matches!(measurement.as_ref(), Measurement::Scalar(_)));

    // Cleanup
    remote.disconnect().await?;
}
```

**Test 2: Command Latency**
```rust
#[tokio::test]
async fn test_command_latency_under_10ms() {
    // Setup similar to Test 1

    // Measure round-trip latency for 100 commands
    let mut latencies = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();

        remote.handle_command(InstrumentCommand::GetParameter {
            name: "status".to_string()
        }).await?;

        let latency = start.elapsed();
        latencies.push(latency);
    }

    // Calculate 99th percentile
    latencies.sort();
    let p99 = latencies[99];

    assert!(p99 < Duration::from_millis(10), "P99 latency: {:?}", p99);
}
```

**Test 3: Network Partition Recovery**
```rust
#[tokio::test]
async fn test_network_partition_recovery() {
    // Setup server + remote instrument

    // Verify connected
    assert!(remote.is_connected());

    // Simulate partition: kill server connection
    server.close_session(&remote.session_id).await?;

    // Wait for partition detection (6s timeout)
    tokio::time::sleep(Duration::from_secs(7)).await;

    assert!(!remote.is_connected());

    // Restart server connection availability
    server.allow_reconnections().await?;

    // Wait for reconnection (exponential backoff)
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Verify reconnected
    assert!(remote.is_connected());

    // Verify data streaming resumed
    let measurement = remote.data_stream().await?.recv().await?;
    assert!(measurement.is_some());
}
```

**Test 4: Two DAQ Instances Sharing One Instrument** (Integration Test per bd-63 requirement)

```rust
#[tokio::test]
async fn test_two_daq_instances_sharing_instrument() {
    // DAQ Instance A (Server)
    let server_config = /* ... */;
    let server = InstrumentServer::new(server_config).await?;

    let mock = Arc::new(Mutex::new(MockInstrument::new()));
    server.register_instrument("mock", Arc::clone(&mock)).await?;

    tokio::spawn(async move { server.run().await });

    // DAQ Instance B (Client 1)
    let mut remote_b = RemoteInstrument::new(/* config */);
    remote_b.connect(&settings).await?;
    let mut data_rx_b = remote_b.data_stream().await?;

    // DAQ Instance C (Client 2)
    let mut remote_c = RemoteInstrument::new(/* config */);
    remote_c.connect(&settings).await?;
    let mut data_rx_c = remote_c.data_stream().await?;

    // Verify both clients receive same data
    let measurement_b = data_rx_b.recv().await?;
    let measurement_c = data_rx_c.recv().await?;

    // Timestamps should be identical (same source)
    assert_eq!(
        measurement_b.timestamp(),
        measurement_c.timestamp()
    );

    // Verify command from Client B affects Client C's view
    remote_b.handle_command(InstrumentCommand::SetParameter {
        name: "sample_rate_hz".to_string(),
        value: "2000.0".to_string(),
    }).await?;

    // Both clients should see updated sample rate in subsequent data
    let measurement_b_after = data_rx_b.recv().await?;
    let measurement_c_after = data_rx_c.recv().await?;

    // (Verification logic depends on MockInstrument implementation)
}
```

### 9.2 Performance Benchmarking

**Benchmark Suite** (`benches/network_latency.rs`):

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_command_latency(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("remote_command_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let remote = /* setup */;

            let start = Instant::now();
            remote.handle_command(black_box(InstrumentCommand::GetParameter {
                name: "status".to_string()
            })).await.unwrap();
            start.elapsed()
        });
    });
}

fn benchmark_data_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("remote_data_streaming", |b| {
        b.to_async(&rt).iter(|| async {
            let remote = /* setup */;
            let mut data_rx = remote.data_stream().await.unwrap();

            // Measure throughput: measurements/sec
            let start = Instant::now();
            let mut count = 0;

            loop {
                if data_rx.recv().await.is_ok() {
                    count += 1;
                }

                if start.elapsed() > Duration::from_secs(1) {
                    break;
                }
            }

            count // measurements per second
        });
    });
}

criterion_group!(benches, benchmark_command_latency, benchmark_data_throughput);
criterion_main!(benches);
```

## 10. Error Handling

### 10.1 DaqError Extensions

**Add networking variants** (`src/error.rs`):

```rust
#[derive(Error, Debug)]
pub enum DaqError {
    // ... existing variants ...

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Connection timeout: {0}")]
    ConnectionTimeout(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Remote instrument error: {0}")]
    RemoteInstrument(String),
}
```

### 10.2 Error Propagation

**Server-side error handling**:
```rust
async fn handle_control_message(&self, session: &ClientSession, msg: ControlMessage) -> Result<()> {
    match msg.type {
        ControlMessageType::CommandRequest => {
            let cmd_req = /* deserialize */;

            // Handle command, catch errors
            let result = self.execute_command(session, cmd_req).await;

            let response = match result {
                Ok(data) => CommandResponse {
                    success: true,
                    result: data,
                    error_message: String::new(),
                },
                Err(e) => {
                    log::error!("Command execution failed: {}", e);
                    CommandResponse {
                        success: false,
                        result: String::new(),
                        error_message: e.to_string(),
                    }
                }
            };

            self.send_response(session, response).await?;
        }
        // ...
    }

    Ok(())
}
```

**Client-side error handling**:
```rust
async fn send_control_message<T>(&self, msg: T) -> Result<CommandResponse> {
    // Attempt send with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        self.send_internal(msg)
    ).await;

    match result {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(e)) => {
            log::error!("Control message failed: {}", e);

            // Check if network partition
            if is_network_error(&e) {
                self.handle_network_partition().await?;
            }

            Err(e)
        }
        Err(_timeout) => {
            log::warn!("Control message timeout, checking connection");
            Err(DaqError::ConnectionTimeout("Command timeout".to_string()))
        }
    }
}
```

## 11. Implementation Roadmap

### Phase 1: Core Protocol & Server (1 week)
1. Define FlatBuffers schema (`protocol.fbs`)
2. Implement FlatBuffers code generation in `build.rs`
3. Implement `InstrumentServer` skeleton
4. Add JWT authentication
5. Implement control channel message handling
6. Unit tests for server components

### Phase 2: Remote Instrument Client (1 week)
1. Implement `RemoteInstrument` struct
2. Implement `Instrument` trait methods
3. Add control channel client logic
4. Add data channel client logic
5. Implement heartbeat mechanism
6. Unit tests for client components

### Phase 3: Network Partition Recovery (3 days)
1. Implement reconnection logic with exponential backoff
2. Add session buffering on server
3. Implement sequence number tracking
4. Test partition recovery scenarios

### Phase 4: Security & TLS (2 days)
1. Add TLS support (feature flag: `networking_tls`)
2. Implement RBAC authorization checks
3. Add rate limiting
4. Security audit

### Phase 5: Integration & Testing (1 week)
1. Write integration tests (Test 1-4 above)
2. Performance benchmarking (latency, throughput)
3. Stress testing (many clients, large images)
4. Documentation and examples

### Phase 6: Documentation & Handoff (2 days)
1. Update CLAUDE.md with networking usage
2. Create example configurations
3. Write user guide for remote instrument setup
4. Update ADR (Architecture Decision Records)

**Total Estimated Time**: 3-4 weeks

## 12. Open Questions & Future Work

### 12.1 Open Questions

1. **JWT Token Management**: How should clients obtain JWT tokens? (Out of scope - assume external auth service)
2. **Multi-tenancy**: Should server support multiple independent DAQ applications? (Future work)
3. **Discovery Protocol**: Should clients auto-discover servers via mDNS/Zeroconf? (Future enhancement)
4. **HTTP API**: Should server expose REST API for instrument status queries? (Nice-to-have)

### 12.2 Future Enhancements (Post-Phase 3A)

1. **Compression**: Add optional zstd compression for large image data (trade latency for bandwidth)
2. **Multiplexing**: Multiplex multiple instruments over single WebSocket pair (reduce connection overhead)
3. **Priority Queues**: Separate high/low priority command channels
4. **Load Balancing**: Distribute instrument access across multiple servers
5. **Observability**: Prometheus metrics for latency, throughput, error rates
6. **Web UI**: Browser-based remote instrument control (WebAssembly GUI)

## 13. References

### 13.1 Industry Standards

- **HiSLIP**: LXI High-Speed LAN Instrument Protocol Specification (IVI Foundation)
- **VXI-11**: VMEbus Extensions for Instrumentation TCP/IP Instrument Protocol
- **SCPI**: Standard Commands for Programmable Instruments (IEC 60488-2)
- **WebSocket**: RFC 6455 - The WebSocket Protocol
- **JWT**: RFC 7519 - JSON Web Token
- **TLS 1.3**: RFC 8446 - Transport Layer Security Version 1.3

### 13.2 Rust Crates

- `tokio-tungstenite`: WebSocket implementation for Tokio
- `flatbuffers`: Zero-copy serialization
- `jsonwebtoken`: JWT validation
- `tokio-native-tls`: TLS support for Tokio

### 13.3 Research & Best Practices

- Perplexity Deep Research: "Remote scientific instrument control protocols" (2025-10-19)
- DynExp: Modern C++ DAQ framework with gRPC networking
- LabVIEW: Network-published shared variables pattern
- HiSLIP dual-channel architecture analysis

## 14. Consensus Summary

**Multi-Model Consensus Results** (Gemini 2.5 Pro, confidence: 9/10):

**MANDATORY**:
- Dual WebSocket model (HiSLIP pattern) to guarantee <10ms command latency
- FlatBuffers for zero-copy serialization (aligns with Arc<Measurement>)

**CRITICAL**:
- WebSocket over gRPC (simpler, lower latency, better async Rust support)
- 2s heartbeat / 6s timeout (rapid partition detection for DAQ reliability)

**BEST PRACTICE**:
- JWT in `Sec-WebSocket-Protocol` header (standard, secure, efficient)

**RATIONALE**:
- Single WebSocket would hit performance wall due to head-of-line blocking
- Protobuf requires copy, betraying zero-copy architectural principle
- gRPC's HTTP/2 overhead jeopardizes <10ms target
- Engineering discipline required: avoid blocking operations on hot path

---

**Document Version**: 1.0
**Status**: Ready for Implementation
**Next Steps**: Begin Phase 1 implementation (Core Protocol & Server)
