# Instrument Control and Integration Guide (V4 Architecture)

## Overview

This guide covers implementing instrument control capabilities for the scientific data acquisition application under the V4 architecture. The core of V4 instrument control is built around **Kameo actors**, where each instrument is an independent, supervised actor. This design ensures robustness, concurrency, and clear separation of concerns.

## 1. Core Instrument Architecture: Kameo Actors

In the V4 architecture, every instrument is encapsulated within its own Kameo actor. This actor manages the instrument's state, connection, and communication with the hardware.

### Key Concepts:

*   **Instrument Actor:** A self-contained Kameo actor responsible for a single physical instrument. It handles connection, configuration, data acquisition, and command execution.
*   **`InstrumentManagerActor`:** A central Kameo actor that supervises and orchestrates all individual instrument actors. It receives high-level commands (e.g., "connect instrument X") and forwards them to the appropriate instrument actor. It also aggregates data from instruments and distributes it to other parts of the system (e.g., GUI, storage).
*   **Messages:** All communication between actors (and from the outside world to actors) happens via asynchronous messages.
*   **`RecordBatch` Data:** Instrument actors produce measurement data as `apache/arrow-rs` `RecordBatch`es.

### Example: Base Instrument Actor Structure

```rust
use kameo::{Actor, Context, Message, ActorRef};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Instant;
use arrow::record_batch::RecordBatch;

// Define messages that an Instrument Actor can receive
#[derive(Message)]
pub struct Connect(pub String); // Connection string/address
#[derive(Message)]
pub struct Disconnect;
#[derive(Message)]
pub struct Configure(pub HashMap<String, serde_json::Value>);
#[derive(Message)]
pub struct StartAcquisition;
#[derive(Message)]
pub struct StopAcquisition;
#[derive(Message)]
#[rtype(result = String)] // Expect a string response
pub struct SendCommand(pub String);
#[derive(Message)]
#[rtype(result = RecordBatch)] // Expect a RecordBatch of data
pub struct ReadData;
#[derive(Message)]
#[rtype(result = InstrumentStatus)]
pub struct GetStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentStatus {
    pub id: String,
    pub connected: bool,
    pub acquiring: bool,
    pub error_state: Option<String>,
    pub last_data_timestamp: Option<Instant>,
    pub data_rate_hz: f64,
}

// Trait for instrument-specific logic (optional, for common interfaces)
#[async_trait]
pub trait InstrumentHardware: Send + Sync {
    async fn connect(&mut self, connection_string: &str) -> anyhow::Result<()>;
    async fn disconnect(&mut self) -> anyhow::Result<()>;
    async fn send_command(&mut self, command: &str) -> anyhow::Result<String>;
    async fn read_raw_data(&mut self) -> anyhow::Result<Vec<u8>>; // Raw bytes
    async fn is_connected(&self) -> bool;
    // ... other hardware-specific methods
}

pub struct GenericInstrumentActor<H: InstrumentHardware> {
    id: String,
    config: serde_json::Value, // Full instrument config
    hardware: H,
    status: InstrumentStatus,
    // Channel to send acquired data to the InstrumentManager
    data_publisher: ActorRef<InstrumentManagerActor>,
}

impl<H: InstrumentHardware + 'static> Actor for GenericInstrumentActor<H> {
    type State = Self;
    type Supervision = kameo::SupervisionStrategy;

    fn new() -> Self::State {
        // This would typically be initialized with actual config/hardware
        unimplemented!("InstrumentActor must be spawned with initial state")
    }
}

// Implement message handlers for the instrument actor
impl<H: InstrumentHardware + 'static> Message<Connect> for GenericInstrumentActor<H> {
    type Result = anyhow::Result<()>;

    async fn handle(&mut self, message: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        tracing::info!("Instrument {} connecting to {}", self.id, message.0);
        self.hardware.connect(&message.0).await?;
        self.status.connected = true;
        tracing::info!("Instrument {} connected", self.id);
        Ok(())
    }
}

impl<H: InstrumentHardware + 'static> Message<StartAcquisition> for GenericInstrumentActor<H> {
    type Result = anyhow::Result<()>;

    async fn handle(&mut self, _message: StartAcquisition, ctx: &mut Context<Self>) -> Self::Result {
        tracing::info!("Instrument {} starting acquisition", self.id);
        self.status.acquiring = true;
        self.status.last_data_timestamp = Some(Instant::now());

        // Spawn a continuous data acquisition task
        let self_ref = ctx.actor_ref().clone();
        let data_publisher = self.data_publisher.clone();
        let instrument_id = self.id.clone();

        ctx.spawn_task(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Simulate acquisition interval
                match self_ref.send(ReadData).await {
                    Ok(record_batch) => {
                        // Publish data to InstrumentManager
                        let _ = data_publisher.send(InstrumentManagerMessage::NewData(instrument_id.clone(), record_batch)).await;
                    },
                    Err(e) => {
                        tracing::error!("Error reading data from instrument {}: {:?}", instrument_id, e);
                        // Handle error, potentially stop acquisition or report to manager
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}

impl<H: InstrumentHardware + 'static> Message<ReadData> for GenericInstrumentActor<H> {
    type Result = RecordBatch;

    async fn handle(&mut self, _message: ReadData, _ctx: &mut Context<Self>) -> Self::Result {
        let raw_data = self.hardware.read_raw_data().await.expect("Failed to read raw data");
        // Convert raw_data to Arrow RecordBatch
        // This conversion logic would be instrument-specific
        RecordBatch::new_empty(Arc::new(arrow::datatypes::Schema::new(vec![]))) // Placeholder
    }
}

impl<H: InstrumentHardware + 'static> Message<GetStatus> for GenericInstrumentActor<H> {
    type Result = InstrumentStatus;

    async fn handle(&mut self, _message: GetStatus, _ctx: &mut Context<Self>) -> Self::Result {
        self.status.clone()
    }
}
```

## 2. Instrument Manager Actor

The `InstrumentManagerActor` is responsible for:
*   Spawning and supervising instrument actors.
*   Routing commands to the correct instrument actor.
*   Aggregating and distributing `RecordBatch` data from all instruments.

```rust
use kameo::{Actor, Context, Message, ActorRef};
use std::collections::HashMap;
use arrow::record_batch::RecordBatch;

// Messages for the InstrumentManagerActor
#[derive(Message)]
pub enum InstrumentManagerMessage {
    RegisterInstrument(String, ActorRef<GenericInstrumentActor<dyn InstrumentHardware>>),
    ConnectInstrument(String, String), // id, connection_string
    StartAcquisition(String), // id
    NewData(String, RecordBatch), // instrument_id, data
    // ... other commands
}

pub struct InstrumentManagerActor {
    instruments: HashMap<String, ActorRef<GenericInstrumentActor<dyn InstrumentHardware>>>,
    // Channels/ActorRefs to send data to GUI, Storage, Processors
    data_subscribers: Vec<ActorRef<dyn DataSubscriber>>, // Example
}

impl Actor for InstrumentManagerActor {
    type State = Self;
    type Supervision = kameo::SupervisionStrategy;

    fn new() -> Self::State {
        InstrumentManagerActor {
            instruments: HashMap::new(),
            data_subscribers: Vec::new(),
        }
    }
}

impl Message<InstrumentManagerMessage> for InstrumentManagerActor {
    type Result = ();

    async fn handle(&mut self, message: InstrumentManagerMessage, _ctx: &mut Context<Self>) -> Self::Result {
        match message {
            InstrumentManagerMessage::RegisterInstrument(id, actor_ref) => {
                tracing::info!("Registering instrument: {}", id);
                self.instruments.insert(id, actor_ref);
            },
            InstrumentManagerMessage::ConnectInstrument(id, connection_string) => {
                if let Some(instrument_ref) = self.instruments.get(&id) {
                    let _ = instrument_ref.send(Connect(connection_string)).await;
                } else {
                    tracing::warn!("Attempted to connect unknown instrument: {}", id);
                }
            },
            InstrumentManagerMessage::StartAcquisition(id) => {
                if let Some(instrument_ref) = self.instruments.get(&id) {
                    let _ = instrument_ref.send(StartAcquisition).await;
                } else {
                    tracing::warn!("Attempted to start acquisition on unknown instrument: {}", id);
                }
            },
            InstrumentManagerMessage::NewData(instrument_id, record_batch) => {
                // Distribute data to all subscribers
                for subscriber in &self.data_subscribers {
                    let _ = subscriber.send(DataSubscriberMessage::ReceiveData(instrument_id.clone(), record_batch.clone())).await;
                }
            }
            // ... handle other messages
        }
    }
}
```

## 3. Implementing Specific Instrument Hardware

Each instrument type will have its own implementation of the `InstrumentHardware` trait.

### Example: SCPI Instrument Hardware

```rust
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::Duration;

pub struct ScpiHardware {
    connection: Option<TcpStream>,
    address: String,
    port: u16,
    timeout: Duration,
    termination: String,
}

#[async_trait]
impl InstrumentHardware for ScpiHardware {
    async fn connect(&mut self, connection_string: &str) -> anyhow::Result<()> {
        let parts: Vec<&str> = connection_string.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid connection string format. Expected 'address:port'");
        }
        self.address = parts[0].to_string();
        self.port = parts[1].parse()?;

        let addr = format!("{}:{}", self.address, self.port);
        let stream = tokio::time::timeout(self.timeout, TcpStream::connect(&addr)).await??;
        self.connection = Some(stream);
        tracing::info!("SCPI Hardware connected to {}", addr);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(stream) = self.connection.take() {
            stream.shutdown().await?;
        }
        tracing::info!("SCPI Hardware disconnected");
        Ok(())
    }

    async fn send_command(&mut self, command: &str) -> anyhow::Result<String> {
        let stream = self.connection.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let cmd_with_term = format!("{}{}", command, self.termination);
        tokio::time::timeout(self.timeout, stream.write_all(cmd_with_term.as_bytes())).await??;

        if command.ends_with('?') {
            let mut buffer = vec![0u8; 1024];
            let bytes_read = tokio::time::timeout(self.timeout, stream.read(&mut buffer)).await??;
            Ok(String::from_utf8_lossy(&buffer[..bytes_read]).trim().to_string())
        } else {
            Ok("".to_string())
        }
    }

    async fn read_raw_data(&mut self) -> anyhow::Result<Vec<u8>> {
        let stream = self.connection.as_mut().ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        let mut buffer = vec![0u8; 4096]; // Example buffer size
        let bytes_read = tokio::time::timeout(self.timeout, stream.read(&mut buffer)).await??;
        Ok(buffer[..bytes_read].to_vec())
    }

    async fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}
```

## 4. Integration with Configuration

Instrument configurations will be loaded via `figment` and passed to the `InstrumentManagerActor` during startup, which then uses them to spawn and configure individual instrument actors.

## 5. Error Handling

Errors in instrument communication or processing will be handled using Rust's `anyhow::Result` and `thiserror` for custom error types, with detailed logging via `tracing`. Kameo's supervision strategies will ensure that instrument actors can be restarted or gracefully shut down upon critical errors.

This instrument control guide provides a comprehensive foundation for integrating various types of scientific instruments into your Rust DAQ application, leveraging the robustness and concurrency of the Kameo actor model and the efficiency of Arrow data.