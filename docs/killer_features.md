# Killer Features - Strategic Advantages Over DynExp/PyMODAQ

## KF1: Headless-First Remote Architecture
type: epic
priority: P0
parent: bd-kal8
description: |
  Implement client-server split with autonomous daemon core.

  PROBLEM WITH MONOLITHIC FRAMEWORKS:
  - DynExp/PyMODAQ are desktop apps
  - Cleanroom/hazardous zone access requires VNC (laggy, insecure)
  - GUI crash kills entire experiment

  SOLUTION:
  - **Daemon Core**: Standalone Rust binary on lab computer
  - **Client UI**: Separate app (potentially WebAssembly)
  - Communication: gRPC or WebSocket

  ADVANTAGES:
  - Safety: GUI crash doesn't kill experiment
  - Remote: Monitor from anywhere without VNC lag
  - Multi-user: Multiple clients can connect
  - Zero trust: Lab computer isolated, API-only access

  UNBLOCKS:
  - Remote experiment monitoring
  - Multi-user collaboration
  - Fault-tolerant operation

## KF2: Hot-Swappable Logic via Embedded Scripting
type: epic
priority: P0
parent: bd-kal8
deps: bd-kal8.4
description: |
  Embed Rhai/Lua scripting for on-the-fly experiment logic changes.

  PROBLEM WITH COMPILED LANGUAGES:
  - Edit-Compile-Run cycle too slow for science
  - Python wins here despite performance issues

  SOLUTION:
  - Embed Rhai or mlua in Rust core
  - Expose InstrumentHandle to script engine
  - Heavy lifting stays in Rust, logic is scripted

  EXAMPLE SCRIPT (Rhai):
  ```rhai
  for i in 0..100 {
      stage.move_abs(start_pos + i * 0.1);
      camera.snap();
      sleep(0.1);
  }
  ```

  ADVANTAGES:
  - Performance: Critical paths in Rust
  - Safety: Sandboxed execution with timeouts
  - Flexibility: Change logic without recompile

  BUILDS ON: bd-kal8.4 (Pillar 4 Scripting)

## KF3: Time-Travel Data Stream (Memory-Mapped Ring Buffer)
type: epic
priority: P0
parent: bd-kal8
description: |
  Implement memory-mapped ring buffer for live data replay.

  PROBLEM WITH CURRENT FRAMEWORKS:
  - HDF5 files locked during acquisition
  - Can't analyze while experiment runs
  - Slow disk I/O for live analysis

  SOLUTION:
  - Allocate shared memory buffer (4GB) for last N minutes
  - Store in Arrow format
  - Background writer persists to disk
  - Python can attach via pyarrow (zero-copy)

  FEATURES:
  - **Instant Replay**: Scroll back 5 minutes in GUI
  - **Live Analysis**: Python ML/AI on live data stream
  - **Zero-Copy**: No serialization overhead

  ADVANTAGES:
  - Rust speed + Python ecosystem
  - Non-blocking analysis
  - Time-travel debugging

  BUILDS ON: bd-kal8.3 (Arrow batching)

## KF4: Capability-Based Hardware Traits (Atomic Capabilities)
type: epic
priority: P0
parent: bd-kal8
deps: bd-kal8.2,bd-kal8.3
description: |
  Replace monolithic instrument traits with atomic capabilities.

  PROBLEM WITH MONOLITHIC TRAITS:
  - Camera trait assumes all cameras have all features
  - Runtime errors when feature missing (Python)
  - Hard to compose experiments generically

  SOLUTION:
  - **Atomic Capabilities**: Triggerable, Cooled, RoiSelectable, FrameProducer
  - **Composition**: Instrument implements capabilities it has
  - **Generic Experiments**: Accept any T: TemperatureSensor + VoltageSource

  EXAMPLE:
  ```rust
  fn temp_dependent_measurement<T, V>(
      temp_sensor: T,
      voltage_source: V
  ) -> Result<Data>
  where
      T: TemperatureSensor,
      V: VoltageSource
  {
      // Works with ANY compatible hardware
  }
  ```

  ADVANTAGES:
  - **Compile-Time Checks**: Catch missing features at build time
  - **Generic Modules**: Experiment code hardware-agnostic
  - **Type Safety**: Rust's superpowerAPPLIES TO:
  - V3 trait redesign (bd-kal8.2)
  - Instrument layer architecture
