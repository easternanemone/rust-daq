# Pillar 2: Adopt V3 Direct Async - Task Breakdown

## P2.1: Analyze Kameo vs V3 Direct Async Performance
type: task
priority: P0
parent: bd-kal8.2
description: |
  Benchmark Kameo actor message passing vs V3 direct async method calls.

  TESTS:
  - Command latency (set voltage, move motor)
  - Throughput (commands/second)
  - Memory overhead (actor mailboxes vs channels)
  - Complexity (LOC for same functionality)

  ACCEPTANCE:
  - Data-driven decision documented
  - If Kameo < 50μs overhead: Keep for cluster
  - If Kameo > 100μs overhead: Delete entirely

## P2.2: Migrate V4 SCPI Actor to V3 Pattern
type: task
priority: P0
parent: bd-kal8.2
deps: bd-kal8.2.1
description: |
  Convert v4-daq/actors/scpi.rs to V3 Direct Async pattern.

  ERRORS TO FIX:
  - scpi.rs:437 (module spawn is private)
  - actor.rs:27 (spawn module definition)

  V3 PATTERN:
  - ScpiInstrument struct with async methods
  - tokio::spawn for task management
  - mpsc channel for commands (not Actor mailbox)

  ACCEPTANCE:
  - scpi.rs compiles with V3 trait
  - No kameo dependencies
  - Command latency < 100μs

## P2.3: Migrate ESP300 to V3 MotionController
type: task
priority: P0
parent: bd-kal8.2
deps: bd-kal8.2.2
description: |
  Convert esp300.rs to implement V3 MotionController trait.

  ERRORS TO FIX:
  - esp300.rs:45 (unresolved import crate::core_v3::MotionController)
  - Multiple unused imports (SerialAdapter, chrono, HashMap, Duration)

  IMPLEMENTATION:
  - Use daq_core::MotionController trait
  - V3 async methods (no Actor messages)
  - Direct hardware adapter calls

  ACCEPTANCE:
  - esp300.rs compiles
  - Implements daq_core::MotionController
  - Hardware tests pass (bd-38fa)

## P2.4: Migrate MaiTai and Newport to V3 Traits
type: task
priority: P0
parent: bd-kal8.2
deps: bd-kal8.2.2
description: |
  Fix V3 trait implementations for laser and power meter.

  ERRORS TO FIX:
  - maitai.rs:405 (get_shutter return type)
  - maitai.rs:406 (missing set_wavelength, get_wavelength, enable_output)
  - newport_1830c.rs:371 (missing power_range, set_power_range)

  ACCEPTANCE:
  - All trait methods implemented
  - Return types match V3 trait signatures
  - Hardware tests pass (bd-cqpl, bd-7sma)

## P2.5: Fix PVCAM V3 Camera Trait Implementation
type: task
priority: P0
parent: bd-kal8.2
deps: bd-kal8.2.2
description: |
  Complete PVCAM V3 Camera trait implementation.

  ERRORS TO FIX:
  - pvcam.rs:874-1270 (missing snap_frame, start/stop_acquisition, roi, exposure_time, set_exposure_time)
  - pvcam.rs:1212,1233,1255 (lifetime parameter mismatches)

  ACCEPTANCE:
  - All Camera trait methods implemented
  - Lifetime parameters correct
  - Hardware tests pass (bd-s76y)

## P2.6: Eliminate All Kameo Dependencies (If Decision is Delete)
type: task
priority: P0
parent: bd-kal8.2
deps: bd-kal8.2.1,bd-kal8.2.2,bd-kal8.2.3,bd-kal8.2.4,bd-kal8.2.5
description: |
  Remove Kameo from project IF performance analysis shows no benefit.

  CONDITIONAL ON: P2.1 decision to delete Kameo

  DELETE:
  - kameo dependency from Cargo.toml
  - v4-daq/actors/ directory
  - All Actor message types
  - Supervision code (replace with JoinSet)

  ACCEPTANCE:
  - cargo tree shows no kameo dependency
  - All instruments use V3 pattern
  - Compilation errors < 30
