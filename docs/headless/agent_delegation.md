# Agent Delegation Structure

## Cleaner Agent Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Agent specializing in aggressive code deletion and cleanup.

  RESPONSIBILITIES:
  - Execute Task A (The Reaper)
  - Delete V1/V2/V4 architectures
  - Clean up Cargo.toml dependencies
  - Remove dead imports and references

  FOCUS FILES:
  - src/app_actor.rs
  - v4-daq/ workspace
  - crates/daq-core/
  - src/adapters/v2_adapter.rs
  - src/instrument/registry_v2.rs

  DELEGATION STRATEGY:
  - Verify no active references before deletion
  - Run cargo check after each deletion
  - Document what was removed and why

  TASKS ASSIGNED:
  - bd-9si6 (Task A: The Reaper)

## Architect Agent Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Agent specializing in trait design and type system architecture.

  RESPONSIBILITIES:
  - Execute Task B (Trait Consolidation)
  - Define atomic capability traits
  - Ensure async_trait patterns correct
  - Design composable hardware interfaces

  FOCUS FILES:
  - src/hardware/capabilities.rs
  - Migration from src/core_v3.rs

  DELEGATION STRATEGY:
  - Follow reference implementation exactly
  - Use anyhow::Result for all methods
  - Document trait composition patterns
  - Provide usage examples

  TASKS ASSIGNED:
  - bd-bm03 (Task B: Trait Consolidation)

## Driver Agent Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Agent specializing in hardware driver implementation.

  RESPONSIBILITIES:
  - Execute Task C (Mock Driver)
  - Implement Movable, Triggerable, FrameProducer for mocks
  - Write comprehensive hardware tests
  - Ensure tokio async patterns correct

  FOCUS FILES:
  - src/hardware/mock.rs
  - tests/mock_hardware.rs

  DELEGATION STRATEGY:
  - Use tokio::time::sleep (not std::thread::sleep)
  - Add println! logs for debugging
  - Test all async methods
  - Verify RwLock usage correct

  TASKS ASSIGNED:
  - bd-wsaw (Task C: Mock Driver Implementation)

## Scripting Agent Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Agent specializing in Rhai scripting integration.

  RESPONSIBILITIES:
  - Execute Tasks D, E, F (Rhai setup, bindings, CLI)
  - Embed Rhai engine correctly
  - Bridge async Rust ↔ sync Rhai
  - Implement safety limits (operation count)

  FOCUS FILES:
  - src/scripting/engine.rs
  - src/scripting/bindings.rs
  - src/main.rs (CLI rewrite)
  - examples/simple_scan.rhai

  DELEGATION STRATEGY:
  - Use tokio::task::block_in_place for sync/async bridge
  - Safety callback: terminate after 10k operations
  - Test infinite loop handling
  - Verify clap CLI works correctly

  TASKS ASSIGNED:
  - bd-jypq (Task D: Rhai Setup)
  - bd-m9bs (Task E: Hardware Bindings)
  - bd-hiu6 (Task F: CLI Rewrite)

## Network Agent Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Agent specializing in gRPC and network protocols.

  RESPONSIBILITIES:
  - Execute Tasks G, H, I (Proto, Server, Client)
  - Define gRPC service interface
  - Implement ControlService
  - Build Python client prototype

  FOCUS FILES:
  - src/network/proto/daq.proto
  - src/network/server.rs
  - clients/python/daq_client.py
  - build.rs (tonic-build setup)

  DELEGATION STRATEGY:
  - Use tonic for gRPC
  - Implement streaming RPCs correctly
  - Handle client disconnects gracefully
  - Test with Python grpcio

  TASKS ASSIGNED:
  - bd-3z3z (Task G: API Definition)
  - bd-8gsx (Task H: gRPC Server)
  - bd-2kon (Task I: Client Prototype)

## Data Agent Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Agent specializing in high-performance data structures.

  RESPONSIBILITIES:
  - Execute Tasks J, K (Ring Buffer, HDF5 Writer)
  - Implement memory-mapped ring buffer
  - Create Arrow ↔ HDF5 translation layer
  - Ensure zero-copy operations

  FOCUS FILES:
  - src/data/ring_buffer.rs
  - src/data/hdf5_writer.rs

  DELEGATION STRATEGY:
  - Use #[repr(C)] for cross-language compatibility
  - Atomic operations for lock-free ring buffer
  - THE MULLET STRATEGY: Arrow in front (fast), HDF5 in back (compatible)
  - Never expose Arrow to script layer (only f64/Vec<f64>)

  KEY PRINCIPLE:
  Scientists should NEVER see Arrow. They get:
  - Input: f64 values in Rhai scripts
  - Output: Standard HDF5 files

  TASKS ASSIGNED:
  - bd-q2we (Task J: Ring Buffer)
  - bd-fspl (Task K: HDF5 Writer)
