# Kameo Integration Architectural Plan

## 1. Introduction

This document outlines the architectural changes required to migrate the `rust-daq` application from its custom `tokio`-based actor system to the `kameo` actor framework. The goal of this migration is to leverage a mature, feature-rich framework to improve the project's long-term maintainability, scalability, and robustness.

## 2. Current Architecture

The existing architecture is a custom, lightweight actor-like system built on `tokio`.

-   **`DaqManagerActor`**: A central struct that runs in a dedicated `tokio` task. It holds all application state, including the list of active instruments.
-   **State Management**: All state modifications are funneled through the `DaqManagerActor` as messages, ensuring sequential, synchronous access and avoiding the need for widespread locks.
-   **Message Passing**: A `tokio::mpsc::channel` is used to send `DaqCommand` messages to the `DaqManagerActor`.
-   **Instrument Handling**: Instruments are spawned into their own `tokio` tasks. They communicate data back to the main system via a `tokio::sync::broadcast` channel.
-   **Communication Flow**:
    1.  The GUI (or other clients) sends a `DaqCommand` to the `DaqManagerActor`.
    2.  The `DaqManagerActor` processes the command and may interact with instrument tasks.
    3.  Instrument tasks perform their work (e.g., taking measurements) and publish data to a broadcast channel.
    4.  The `DaqManagerActor` and potentially other parts of the system subscribe to this broadcast data.

This system is effective but lacks built-in supervision, typed `ask` patterns, and other features provided by a formal actor framework.

## 3. Proposed Kameo-based Architecture

The proposed architecture will replace the custom actor implementation with `kameo` actors and messages.

-   **`DaqManager` Kameo Actor**: The `DaqManagerActor` struct will be refactored to implement the `kameo::Actor` trait. It will become a formal `kameo` actor, managing its state internally.
-   **`Instrument` Kameo Actor**: A new generic `InstrumentActor<T: Instrument>` will be created. This actor will wrap any object that implements the `daq_core::Instrument` trait. It will be responsible for managing the instrument's lifecycle and handling communication.
-   **Typed Messages**: The `DaqCommand` enum will be replaced with strongly-typed message structs that implement `kameo::message::Message`. This enables the use of `ask` patterns and provides compile-time type safety.
-   **Communication Flow**:
    1.  The GUI will obtain a `pid::Pid<DaqManager>` for the main manager actor.
    2.  To interact with the system, the GUI will use `pid.ask(MyMessage).await`, which returns a `Result`. This replaces the fire-and-forget `mpsc::Sender`.
    3.  The `DaqManager` actor will spawn `InstrumentActor`s when an "add instrument" command is received. It will store the `Pid<InstrumentActor>` for each instrument.
    4.  The `DaqManager` can then communicate with instrument actors using their Pids.
    5.  Data streaming from instruments will be handled by `kameo`'s stream integration, piping the instrument's data stream directly into the actor's mailbox for processing.

## 4. Migration Steps

### Step 1: Add `kameo` Dependency

Add `kameo` to the `Cargo.toml`:

```toml
[dependencies]
# ...
kameo = { version = "0.1", features = ["full"] }
```

### Step 2: Define `DaqManager` Actor and Messages

Refactor `src/app_actor.rs` and `src/messages.rs`.

1.  **Create Typed Messages**: Convert variants of the `DaqCommand` enum into distinct message structs.

    ```rust
    // Before: src/messages.rs
    pub enum DaqCommand {
        AddInstrument(InstrumentConfig),
        // ...
    }

    // After: src/messages.rs
    use kameo::message::Message;

    #[derive(Message)]
    #[response(Result<Pid<InstrumentActor>>)]
    pub struct AddInstrument {
        pub config: InstrumentConfig,
    }
    // ... other messages
    ```

2.  **Implement `kameo::Actor` for `DaqManager`**:

    ```rust
    // Before: src/app_actor.rs
    pub struct DaqManagerActor { /* ... */ }
    impl DaqManagerActor {
        pub async fn run(mut self) { /* loop with mpsc recv */ }
    }

    // After: src/actors/daq_manager.rs
    use kameo::actor::Actor;

    pub struct DaqManager {
        instruments: HashMap<String, Pid<InstrumentActor>>,
    }

    #[async_trait]
    impl Actor for DaqManager {
        async fn pre_start(&mut self) -> Result<()> {
            println!("DaqManager actor started");
            Ok(())
        }
    }

    // Implement message handlers
    use kameo::message::Handler;

    #[async_trait]
    impl Handler<AddInstrument> for DaqManager {
        async fn handle(&mut self, msg: AddInstrument) -> Result<Pid<InstrumentActor>> {
            // ... logic to spawn an InstrumentActor
        }
    }
    ```

### Step 3: Refactor `main.rs` to Spawn the `DaqManager` Actor

Update the application entry point to use `kameo`.

```rust
// Before: main.rs
let (tx, rx) = mpsc::channel(100);
let manager = DaqManagerActor::new(tx.clone());
tokio::spawn(manager.run());
// ... pass tx to GUI

// After: main.rs
use kameo::actor::ActorRef;

let daq_manager_pid = ActorRef::new(DaqManager::new()).spawn();
// ... pass daq_manager_pid to GUI
```

### Step 4: Create a Generic `InstrumentActor`

Create a new file `src/actors/instrument.rs`.

```rust
// src/actors/instrument.rs
use daq_core::Instrument;
use kameo::actor::Actor;
use kameo::message::StreamHandler;

pub struct InstrumentActor<I: Instrument> {
    instrument: I,
    data_subscribers: Vec<Pid<...>>, // e.g., Pid<DaqManager>
}

#[async_trait]
impl<I: Instrument> Actor for InstrumentActor<I> {
    async fn pre_start(&mut self) -> Result<()> {
        // Connect to the physical instrument
        self.instrument.connect().await?;
        Ok(())
    }
}

// Handle data streaming from the instrument
#[async_trait]
impl<I: Instrument> StreamHandler<I::Data> for InstrumentActor<I> {
    async fn handle(&mut self, data: I::Data) -> Result<()> {
        // Forward data to subscribers
        for sub in &self.data_subscribers {
            sub.tell(data.clone()).await?;
        }
        Ok(())
    }
}
```

### Step 5: Update GUI Communication

Refactor the GUI to use `pid.ask` instead of sending commands over the MPSC channel. This provides a direct, typed response mechanism.

## 5. Benefits of Migration

-   **Robustness**: `kameo` provides supervision trees. If an `InstrumentActor` panics, the `DaqManager` can be notified and decide whether to restart it, without crashing the application.
-   **Clarity and Type Safety**: Using typed messages with `#[response(...)]` makes the communication contract between actors explicit and verified at compile time.
-   **Reduced Boilerplate**: Eliminates the manual `select!` loop for handling messages and managing channels. `kameo` handles the actor's mailbox and message dispatch.
-   **Testability**: Actors can be spawned in isolation during tests, and their Pids can be used to send messages and assert responses, simplifying unit and integration testing.

## 6. Risks and Mitigation

-   **Learning Curve**: The team will need to learn the `kameo` API and actor model concepts.
    -   **Mitigation**: Start with a small proof-of-concept (this branch) and create internal documentation and examples.
-   **Integration Complexity**: There may be unforeseen issues when integrating with the existing `tokio` tasks and GUI.
    -   **Mitigation**: The migration will be done incrementally, starting with one actor at a time, with extensive testing at each step.
-   **Performance**: While `kameo` is highly performant, any abstraction can introduce overhead.
    -   **Mitigation**: Performance benchmarks will be run before and after the migration to ensure there are no significant regressions. The `rust-daq-performance-test.md` guide will be followed.
