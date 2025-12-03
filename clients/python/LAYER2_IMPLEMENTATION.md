# Layer 2 API Implementation Summary

**Issue:** bd-daun.2  
**Status:** Completed  
**Date:** 2025-12-03

## Overview

Implemented the Layer 2 high-level synchronous device API for the rust-daq Python client library. This API provides an Ophyd/Bluesky-style interface for scientists who prefer synchronous, property-based interactions with hardware.

## Architecture

The Layer 2 API sits on top of Layer 1 (AsyncClient) and provides synchronous wrappers using `anyio.from_thread.start_blocking_portal()`.

```
User Code (sync)
       ↓
Layer 2: devices.py (sync API)
       ↓ anyio.from_thread.run()
Layer 1: AsyncClient (async gRPC)
       ↓ gRPC
Daemon (Rust)
```

## Files Created

### `/src/rust_daq/devices.py` (550 lines)

**Classes:**
- `Device`: Base class for all hardware devices
- `Motor(Device)`: For Movable devices (position control)
- `Detector(Device)`: For Readable devices (measurements)
- `Status`: Tracks asynchronous operation completion

**Context Managers:**
- `connect(host, timeout)`: Manages AsyncClient lifecycle
- `run(name, metadata)`: Placeholder for StartRun/StopRun

**Functions:**
- `scan(detectors, motor, start, stop, steps, dwell_time)`: 1D scan with progress bar

### `/tests/test_devices.py` (650 lines)

Comprehensive test suite covering:
- Unit tests for Status class
- Integration tests for all device classes
- Context manager tests
- scan() function tests
- Complete workflow tests

All integration tests are marked with `@pytest.mark.integration` and require a running daemon.

### `/examples/layer2_demo.py`

Demonstrates:
- Property-based motor control
- Detector reading
- Scan execution
- Context manager usage

## Key Design Patterns

### 1. Thread-Local Client Storage

```python
_thread_local = threading.local()

def _get_client() -> AsyncClient:
    if not hasattr(_thread_local, 'client'):
        raise DaqError("No active connection")
    return _thread_local.client
```

Avoids global state while providing transparent access to the client.

### 2. Property-Based Interface

```python
@property
def position(self) -> float:
    client = _get_client()
    return _run_async(client.get_position(self.device_id))

@position.setter
def position(self, value: float):
    self.move(value, wait=True)
```

Provides intuitive `motor.position = 10.0` syntax.

### 3. Async-to-Sync Conversion

```python
def _run_async(coro):
    portal = _get_portal()
    return portal.call(coro)
```

Uses anyio's blocking portal to execute async operations synchronously.

### 4. Optional Dependencies

```toml
[project.optional-dependencies]
scan = [
    "pandas>=1.3",
    "tqdm>=4.60",
]
```

Core functionality works without pandas/tqdm, but scan() benefits from them.

## API Examples

### Basic Motor Control

```python
from rust_daq import connect, Motor

with connect():
    motor = Motor("mock_stage")
    motor.position = 10.0  # Blocking move
    print(motor.position)  # Read position
    print(motor.limits)    # (min, max)
    print(motor.units)     # "mm"
```

### Detector Reading

```python
from rust_daq import connect, Detector

with connect():
    detector = Detector("mock_power_meter")
    value = detector.read()
    print(f"{value} {detector.units}")
```

### Non-Blocking Operations

```python
status = motor.move(20.0, wait=False)
# Do other work...
status.wait()  # Block until complete
```

### 1D Scan

```python
from rust_daq import connect, Motor, Detector, scan

with connect():
    motor = Motor("mock_stage")
    det = Detector("mock_power_meter")
    
    data = scan(
        detectors=[det],
        motor=motor,
        start=0, stop=100, steps=11,
        dwell_time=0.1
    )
    
    print(data)  # pandas DataFrame
```

## Success Criteria Met

✓ **Motor class with position property** - Implemented with get/set  
✓ **Detector class with read() method** - Implemented  
✓ **Context managers (connect, run)** - Both implemented  
✓ **scan() returns DataFrame** - With pandas fallback to dict  
✓ **Status objects for non-blocking ops** - Implemented  
✓ **Comprehensive tests** - 650 lines of tests  
✓ **Documentation complete** - README fully updated  

## Known Limitations

### 1. run() Context Manager

Currently a placeholder. Needs StartRun/StopRun gRPC methods to be implemented in the daemon.

**Current behavior:** Issues a warning and does nothing  
**Future:** Will call client.start_run() and client.stop_run()

### 2. Status Objects

Currently complete immediately because we're using `anyio.from_thread.run()` which blocks.

**Current behavior:** Status.done is always True after creation  
**Future:** Use background tasks for true async status tracking

### 3. No Streaming Support

Layer 2 doesn't yet support streaming device state.

**Future enhancement:** Add `Detector.subscribe(callback)` using async iteration

## Testing

### Run Unit Tests Only

```bash
cd clients/python
pip install -e ".[dev]"
pytest -m "not integration"
```

### Run Integration Tests

Requires daemon running:

```bash
# Terminal 1: Start daemon
cd ../..  # rust-daq root
cargo run --features networking -- daemon --port 50051

# Terminal 2: Run tests
cd clients/python
pytest -m integration
```

### Run All Tests

```bash
pytest
```

## Dependencies

**Required:**
- grpcio >= 1.50
- protobuf >= 4.20
- anyio >= 3.0
- numpy >= 1.20

**Optional (scan support):**
- pandas >= 1.3
- tqdm >= 4.60

**Dev:**
- pytest >= 7.0
- pytest-asyncio >= 0.21
- pytest-mock >= 3.10

## Future Enhancements

1. **True Async Status Objects**
   - Use background tasks via anyio
   - Support callbacks on completion
   - Better cancellation support

2. **Streaming Support**
   - `detector.subscribe(callback)` for real-time updates
   - Async iterator interface
   - Rate limiting

3. **run() Context Manager**
   - Implement StartRun/StopRun gRPC calls
   - Automatic metadata capture
   - Run database integration

4. **Advanced Scan Types**
   - Grid scan (2D)
   - Adaptive scans
   - Interruptible scans
   - Scan composition

5. **Jupyter Integration** (bd-daun.3)
   - IPython display hooks
   - Interactive widgets
   - Live plotting
   - Progress bars in notebooks

## Related Issues

- **Parent:** bd-daun (Epic: Python client library)
- **Sibling:** bd-daun.1 (Layer 1: AsyncClient) - ✓ Complete
- **Next:** bd-daun.3 (Jupyter integration)
- **Next:** bd-daun.4 (Documentation and examples)

## Files Modified

- `/src/rust_daq/__init__.py` - Added Layer 2 exports
- `/pyproject.toml` - Made pandas/tqdm optional
- `/README.md` - Complete rewrite with 3-layer docs

## Verification

```bash
# Syntax check
python3 -m py_compile src/rust_daq/devices.py
python3 -m py_compile tests/test_devices.py

# Import check
python3 -c "from rust_daq import Motor, Detector, scan, connect"

# Demo (requires daemon)
python3 examples/layer2_demo.py
```

## Conclusion

The Layer 2 API successfully provides a scientist-friendly, synchronous interface to the rust-daq daemon. It follows established patterns from Bluesky/Ophyd while being simpler and more Pythonic. The property-based interface makes it ideal for interactive use in IPython, Jupyter notebooks, and simple scripts.
