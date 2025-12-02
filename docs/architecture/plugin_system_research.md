# Plugin System Research & Architecture Proposal

## 1. Current System Analysis

### GUI Architecture
The project currently employs a **Dual-GUI Architecture**, which creates a distinction between local hardware control and remote operation.

1.  **Local/Embedded GUI (`src/gui/`)**:
    *   **Library**: `egui` (Immediate Mode).
    *   **Context**: Embedded directly within the main `rust_daq` binary.
    *   **Hardware Access**: Direct. It communicates with the application state (`DaqApp`) via internal Tokio channels.
    *   **Role**: Primary interface for local configuration, debugging, and direct hardware control.

2.  **Remote Client (`gui/`)**:
    *   **Library**: `Slint` (Declarative, Retained Mode).
    *   **Context**: Standalone binary (`daq-gui`).
    *   **Hardware Access**: Indirect. It interacts with the backend via **gRPC** (`tonic`).
    *   **Role**: Remote monitoring and control client.

### Hardware Abstraction Layer (HAL)
The system uses a **Capability-Based** trait system (`src/hardware/capabilities.rs`) rather than monolithic device objects.

*   **Traits**: `Movable`, `Triggerable`, `ExposureControl`, `FrameProducer`, `Readable`.
*   **Current Implementation**: Hardware drivers (e.g., `esp300.rs`, `pvcam.rs`) manually implement these traits in Rust code. Adding a new device requires writing new Rust structs and recompiling.

## 2. Proposed "Plugin Factory" Architecture

To enable user-defined instruments without recompilation, we propose a **Data-Driven Interpreter** pattern.

### Core Concept
Instead of generating Rust code from configuration (which requires a build step), the system will provide a **Generic Driver** that acts as a runtime interpreter. It loads a definition file (YAML/TOML) and "becomes" that instrument by dynamically mapping the capability traits to the configured commands.

### Architecture Components

#### A. The Instrument Schema (YAML)
Defines the protocol, capabilities, and UI hints.

```yaml
metadata:
  id: "my-sensor-v1"
  driver_type: "serial_scpi"

capabilities:
  readable:
    - name: "temperature"
      command: "READ:TEMP?"
      pattern: "TEMP {value}" # Friendly parsing pattern

ui:
  - type: "readout"
    source: "temperature"
```

#### B. The Generic Driver (`src/hardware/generic/`)
A Rust struct that implements the standard traits (`Readable`, `Movable`) but delegates the logic to the loaded configuration.

*   **`GenericDriver`**: Holds the `AsyncSerialPort` and the `InstrumentConfig`.
*   **`impl Readable for GenericDriver`**: Looks up the command string, sends it, parses the response using the configured pattern, and returns the value.

#### C. The Plugin Registry
A system that scans a specific directory (e.g., `plugins/`) at startup, validates the YAML files, and makes them available to the main application as selectable drivers.

## 3. Implementation Strategy

1.  **Dependency Updates**:
    *   Add `serde_yaml` for parsing definitions.
    *   Add a "template matching" library (e.g., `prse` or custom regex generation) for user-friendly response parsing.

2.  **Phase 1: The Generic Core**:
    *   Create `src/hardware/generic/mod.rs`.
    *   Define the `InstrumentConfig` structs using `serde`.
    *   Implement a basic `GenericDriver` that supports just the `Readable` trait (read-only sensors).

3.  **Phase 2: UI Integration (Egui)**:
    *   Since the main hardware control happens in the `egui` app, create a `GenericPanel` widget.
    *   This widget iterates over the `ui` section of the YAML and renders the corresponding `egui` components (sliders, buttons, labels).

## 4. Trade-offs & Considerations

*   **Performance**: Interpreting commands at runtime adds a negligible overhead compared to the latency of physical hardware communication (Serial/TCP).
*   **Complex Logic**: YAML is poor at defining complex state machines (e.g., "If response is X, wait 5ms, then send Y").
    *   *Mitigation*: Support embedded scripting (Rhai) within the YAML for advanced edge cases, leveraging the existing `rhai` integration.
