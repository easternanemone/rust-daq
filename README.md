# Rust DAQ: Modular Scientific Data Acquisition

A modular, high-performance, and type-safe scientific data acquisition (DAQ) application written in Rust.

## Features

- **Asynchronous Core**: Built on the `tokio` async runtime for high-performance, non-blocking I/O with scientific instruments.
- **Modular & Extensible**: Plugin-based architecture for adding new instruments, data processors, and storage formats.
- **Real-time GUI**: A responsive user interface built with `egui` for real-time data visualization and control.
- **Robust Data Handling**:
  - Ring buffer-based streaming for efficient memory usage.
  - Support for multiple data storage backends (CSV, HDF5, Arrow).
- **Type Safety & Performance**: Leverages Rust's safety guarantees to prevent common bugs in concurrent systems.

## Architecture Overview

The application consists of several key components:

- **Core**: Defines the essential traits (`Instrument`, `DataProcessor`, `StorageWriter`) and data types.
- **Instruments**: Responsible for communicating with hardware. They run on a dedicated `tokio` thread pool.
- **Data Pipeline**: Data from instruments is streamed through `tokio::sync::broadcast` channels. This allows multiple consumers (GUI, processors, storage) to access the data concurrently.
- **GUI**: The `egui`-based interface runs on the main thread and communicates with the async backend via channels.
- **Plugins**: A static plugin system allows for compile-time registration of new components.

## Getting Started

### Prerequisites

- **Rust Toolchain**: Install Rust via [rustup](https://rustup.rs/).
- **System Dependencies**:
  - **HDF5**: To enable the HDF5 storage backend, you need the HDF5 library installed.
    - On Ubuntu/Debian: `sudo apt-get install libhdf5-dev`
    - On macOS: `brew install hdf5`

### Building and Running

1.  **Clone the repository:**
    ```sh
    git clone https://github.com/TheFermiSea/rust-daq.git
    cd rust-daq
    ```

2.  **Build the application:**
    - To build with default features (CSV storage, Serial instruments):
      ```sh
      cargo build --release
      ```
    - To build with all features (HDF5, Arrow, etc.):
      ```sh
      cargo build --release --features full
      ```

3.  **Run the application:**
    ```sh
    cargo run --release
    ```

### Configuration

Application settings can be configured in `config/default.toml`. This includes log levels, instrument parameters, and default storage paths.

## Directory Structure

```
.
├── config/
│   └── default.toml    # Default configuration
├── src/
│   ├── main.rs         # Application entry point
│   ├── lib.rs          # Library root
│   ├── app.rs          # Core application state
│   ├── core.rs         # Core traits and types
│   ├── config.rs       # Configuration loading
│   ├── error.rs        # Custom error types
│   ├── gui.rs          # Egui implementation
│   ├── data/           # Data processing & storage
│   └── instrument/     # Instrument trait and implementations
└── tests/
    └── integration.rs  # Integration tests
```

## How to Add a New Instrument

1.  Create a new file in `src/instrument/`, e.g., `my_instrument.rs`.
2.  Implement the `Instrument` trait from `src/core.rs`.
3.  In `main.rs`, register your new instrument in the `instrument_registry`.
4.  Add any necessary configuration for your instrument to `config/default.toml` and `src/config.rs`.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## License

This project is licensed under the MIT License.
