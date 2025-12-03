# rust-daq-client

Python client library for the rust-daq headless daemon. Provides async-first gRPC access to hardware control, data acquisition, and experiment orchestration.

## Architecture

This client follows a **3-layer architecture**:

- **Layer 0**: Auto-generated protobuf code (in `src/rust_daq/generated/`, created during `pip install`)
- **Layer 1**: `AsyncClient` - Robust async wrapper with error translation and type hints (implemented in bd-daun.1)
- **Layer 2**: High-level synchronous API (coming in bd-daun.2)

## Installation

### Requirements

- Python 3.8+
- rust-daq daemon running (see main project README)

### Install from source

```bash
cd clients/python

# Create virtual environment (recommended)
python3 -m venv .venv
source .venv/bin/activate

# Install in editable mode (auto-generates protobuf code)
pip install -e .

# Or install with development dependencies
pip install -e ".[dev]"
```

The installation automatically:
1. Compiles `proto/daq.proto` using `grpcio-tools`
2. Places generated code in `src/rust_daq/generated/`
3. Fixes import statements for proper package structure

## Quick Start

### Basic Usage (Async)

```python
import anyio
from rust_daq import AsyncClient

async def main():
    # Connect to daemon
    async with AsyncClient("localhost:50051") as client:
        # Get daemon info
        info = await client.get_daemon_info()
        print(f"Daemon version: {info['version']}")

        # List devices
        devices = await client.list_devices()
        for device in devices:
            print(f"Device: {device['id']} ({device['driver_type']})")

        # Control a movable device
        await client.move_absolute("mock_stage", 10.0)
        position = await client.get_position("mock_stage")
        print(f"Current position: {position}")

# Run with anyio (works with asyncio, trio, etc.)
anyio.run(main)
```

### Error Handling

```python
from rust_daq import AsyncClient
from rust_daq.exceptions import DeviceError, CommunicationError, TimeoutError

async with AsyncClient("localhost:50051", timeout=5.0) as client:
    try:
        await client.move_absolute("stage", 100.0)
    except DeviceError as e:
        print(f"Device error: {e.message}")
        print(f"Device ID: {e.device_id}")
    except TimeoutError as e:
        print(f"Operation timed out after {e.timeout_seconds}s")
    except CommunicationError as e:
        print(f"Communication error: {e.message}")
        print(f"gRPC code: {e.grpc_code}")
```

### Streaming Device State

```python
async with AsyncClient("localhost:50051") as client:
    # Stream state updates for all devices at 10 Hz
    async for update in client.stream_device_state(max_rate_hz=10):
        print(f"Device {update['device_id']}: {update['fields']}")
```

## API Reference

### AsyncClient

#### Connection

- `AsyncClient(address="localhost:50051", timeout=10.0)` - Create client
- `async with AsyncClient(...) as client:` - Context manager (recommended)
- `await client.connect()` - Manual connection
- `await client.close()` - Manual disconnect

#### Device Discovery

- `await client.list_devices(capability_filter=None)` - List all devices
- `await client.get_device_state(device_id)` - Get current device state

#### Motion Control (for Movable devices)

- `await client.move_absolute(device_id, position, wait_for_completion=False)`
- `await client.move_relative(device_id, distance, wait_for_completion=False)`
- `await client.get_position(device_id)`

#### Parameter Control

- `await client.set_parameter(device_id, parameter_name, value)`
- `await client.get_parameter(device_id, parameter_name)`

#### Streaming

- `async for update in client.stream_device_state(device_ids=None, max_rate_hz=10):`

### Exceptions

- `DaqError` - Base exception for all client errors
- `DeviceError` - Device-specific errors (not found, operation failed, etc.)
- `CommunicationError` - gRPC communication errors
- `TimeoutError` - Operation timeout (subclass of CommunicationError)
- `ConfigurationError` - Invalid parameters or configuration

## Testing

### Run Unit Tests

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run unit tests (no daemon required)
pytest -m "not integration"

# Run all tests with coverage
pytest --cov=rust_daq
```

### Run Integration Tests

Integration tests require a running rust-daq daemon on `localhost:50051`.

```bash
# Terminal 1: Start daemon
cd /path/to/rust-daq
cargo run --features networking -- daemon --port 50051

# Terminal 2: Run integration tests
cd clients/python
pytest -m integration
```

## Development

### Package Structure

```
clients/python/
├── pyproject.toml          # Package metadata and dependencies
├── setup.py                # Protobuf compilation during install
├── src/
│   └── rust_daq/
│       ├── __init__.py     # Public API exports
│       ├── _version.py     # Version string
│       ├── core.py         # AsyncClient implementation
│       ├── exceptions.py   # Custom exception classes
│       └── generated/      # Auto-generated protobuf (gitignored)
└── tests/
    └── test_client.py      # Unit and integration tests
```

### Regenerate Protobuf Code

Protobuf code is automatically generated during `pip install`. To manually regenerate:

```bash
pip install --force-reinstall --no-deps -e .
```

### Code Quality

```bash
# Format code
black src/ tests/

# Lint
ruff check src/ tests/

# Type checking
mypy src/
```

## Comparison with Legacy Client

The new package structure provides several improvements over the old `daq_client.py`:

| Feature | Legacy | New (v0.1.0) |
|---------|--------|--------------|
| Package structure | Single file | Proper Python package |
| Async support | No | Full async/await with anyio |
| Error handling | Generic RuntimeError | Typed exception hierarchy |
| Type hints | Minimal | Comprehensive |
| Install method | Manual setup.sh | Standard `pip install` |
| Protobuf generation | Manual script | Automatic during install |
| Testing | None | Unit + integration tests |
| Dependencies | requirements.txt | pyproject.toml (PEP 621) |

## Roadmap

- [x] Layer 0: Auto-generated protobuf (bd-daun.1)
- [x] Layer 1: AsyncClient with async API (bd-daun.1)
- [ ] Layer 2: High-level synchronous API (bd-daun.2)
- [ ] Layer 3: Ophyd/Bluesky-compatible device abstraction (bd-daun.3)

## License

Same as rust-daq project.

## Contributing

See main rust-daq project for contribution guidelines.
