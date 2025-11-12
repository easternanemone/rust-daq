# V4 System Architecture

## 1. Overview & Status

**This document describes the new V4 architecture, which is currently under implementation.**

The project is undergoing a complete architectural refactor to unify three previously competing cores (V1, V2, V3) into a single, robust, and maintainable system. The legacy architecture documentation has been archived.

For background on the issues that led to this refactoring, please see [ARCHITECTURAL_FLAW_ANALYSIS.md](./ARCHITECTURAL_FLAW_ANALYSIS.md).

The guiding principles of the V4 architecture are:
- **Clarity and Maintainability**: A single, coherent architecture.
- **Robustness**: Strong compile-time guarantees and supervision via an actor framework.
- **Performance**: High-throughput, low-latency data handling.
- **Interoperability**: Use of industry-standard data formats.

## 2. Core Technologies

The V4 architecture is built upon a stack of modern, best-in-class Rust libraries. The full rationale for these choices is documented in [RUST_LIBRARY_RECOMMENDATIONS.md](./RUST_LIBRARY_RECOMMENDATIONS.md) and [ADDITIONAL_LIBRARY_RESEARCH.md](./ADDITIONAL_LIBRARY_RESEARCH.md).

| Component | Library | Role |
|---|---|---|
| **Concurrency** | [Kameo](https://github.com/jprochazk/kameo) | Manages all instruments as isolated, stateful actors. Provides supervision and lifecycle management. |
| **In-Memory Data** | [Apache Arrow](https://arrow.apache.org/) (`arrow-rs`) | Represents all measurement data (scalars, images, spectra) in a standardized, high-performance columnar format. |
| **Data Storage** | [HDF5](https://www.hdfgroup.org/solutions/hdf5/) (`hdf5-rust`) | Primary format for persistent storage of scientific data, providing hierarchical organization. |
| **Logging** | [Tracing](https://github.com/tokio-rs/tracing) | Provides structured, asynchronous-aware logging and diagnostics across all actors and tasks. |
| **Configuration** | [Figment](https://github.com/SergioBenitez/Figment) | Manages application configuration from multiple sources (e.g., TOML files, environment variables). |
| **GUI Plotting** | [egui_plot](https://github.com/emilk/egui_plot) | Provides native, immediate-mode plotting for the `egui`-based user interface. |
| **Instrument Control**| [visa-rs](https://github.com/TsuITOAR/visa-rs) | Provides safe, high-level bindings to the NI-VISA standard for broad instrument compatibility. |

## 3. High-Level Design

```mermaid
graph LR
    subgraph "User Interface (egui)"
        GUI[GUI w/ egui_plot]
    end

    subgraph "V4 Core (Kameo Actors)"
        subgraph "Supervision"
            InstrumentManager[InstrumentManager Actor]
        end

        subgraph "Instruments"
            InstA[Instrument Actor A<br/>(e.g., Camera)]
            InstB[Instrument Actor B<br/>(e.g., Power Meter)]
            InstC[Instrument Actor C<br/>(...)]
        end

        subgraph "Data Processing"
            StorageActor[Storage Actor<br/>(HDF5 / Zarr)]
            AnalysisActor[Analysis Actor<br/>(Polars)]
        end
    end

    subgraph "Hardware Abstraction Layer"
        HAL_VISA[visa-rs]
        HAL_Serial[tokio-serial]
    end

    %% Flows
    GUI -- Command --> InstrumentManager
    InstrumentManager -- Command --> InstA
    InstrumentManager -- Command --> InstB

    InstA -- Measurement (Arrow) --> InstrumentManager
    InstB -- Measurement (Arrow) --> InstrumentManager

    InstrumentManager -- Data (Arrow) --> StorageActor
    InstrumentManager -- Data (Arrow) --> AnalysisActor
    InstrumentManager -- Data (Arrow) --> GUI

    InstA -.-> HAL_VISA
    InstB -.-> HAL_Serial
```

### 3.1. Instrument Actors

- Each instrument is a self-contained `kameo::Actor`.
- It owns its own state (configuration, connection handle) and runs in its own supervised task.
- Communication with the outside world is exclusively through asynchronous messages. This eliminates the possibility of data races or deadlocks related to instrument state.
- The `InstrumentManager` is responsible for spawning, shutting down, and restarting failed instrument actors.

### 3.2. Data Flow

- All data produced by instruments will be formatted as **Apache Arrow** record batches. This provides a unified data format for scalars, waveforms, images, and spectra.
- Data is published from the instrument actors to the `InstrumentManager`, which then distributes it to other interested actors (like the `StorageActor` or the GUI).
- The `StorageActor` is responsible for writing the Arrow record batches to **HDF5** files.

## 4. Refactoring Plan

The implementation of this new architecture is tracked in the `beads` issue tracker under the main epic **`bd-xvpw`**. The plan is divided into four main phases:
1.  **Phase 0: Foundation:** Define the V4 architecture and create the core crate.
2.  **Phase 1: Vertical Slice:** Implement the first instrument (Newport 1830C) using the full V4 stack to prove the design.
3.  **Phase 2: Instrument Migration:** Migrate all remaining instruments to the V4 actor model in parallel.
4.  **Phase 3: System Migration:** Migrate application-level systems like data processors and the sequencer.
5.  **Phase 4: The Great Purge:** Delete all legacy V1, V2, and V3 code.

This document will be updated as the V4 implementation progresses.
