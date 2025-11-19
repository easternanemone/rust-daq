# Phase 1: Core Clean-Out (Weeks 1-2)

## Phase 1: Core Clean-Out Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Remove V1, V2, V4 artifacts. Stabilize on capability-based architecture.

  OBJECTIVE: Delete vast swaths of legacy code to make room for new architecture.
  TIMELINE: Weeks 1-2
  PARALLELIZABLE: Tasks A, B, C can run simultaneously

  SUCCESS CRITERIA:
  - src/app_actor.rs deleted
  - v4-daq/ workspace deleted
  - crates/daq-core deleted
  - src/hardware/capabilities.rs created with Movable, Triggerable, FrameProducer traits
  - MockStage and MockCamera compile and pass basic tests
  - Project compiles with zero errors

## Task A: The Reaper - Delete Legacy Architectures
type: task
priority: P0
parent: bd-oq51.1
description: |
  Aggressively delete V1, V2, and V4 code. NO backward compatibility.

  FILES TO DELETE:
  - src/app_actor.rs (V2 monolithic actor - ~800 lines)
  - v4-daq/ (entire workspace member with Kameo actors)
  - crates/daq-core/ (V2 legacy traits)
  - src/adapters/v2_adapter.rs
  - src/instrument/registry_v2.rs

  CARGO.TOML UPDATES:
  - Remove v4-daq from workspace members
  - Remove kameo dependency
  - Remove daq-core from dependencies

  COMPILATION ERRORS FIXED:
  - v2_adapter.rs:575-604 (missing trait methods) - FILE DELETED
  - registry_v2.rs:90-124 (trait incompatibilities) - FILE DELETED
  - scpi.rs:437 (module spawn private) - WILL FIX in Task B
  - All V2/V4 import errors - FILES DELETED

  ACCEPTANCE:
  - grep -r "app_actor" src/ returns 0 results
  - grep -r "v2_adapter" src/ returns 0 results
  - v4-daq/ directory does not exist
  - cargo check shows < 50 errors (down from 87)

## Task B: Trait Consolidation - Define Atomic Capabilities
type: task
priority: P0
parent: bd-oq51.1
deps: bd-oq51.1.1
description: |
  Create capability-based trait system for hardware drivers.

  REFERENCE IMPLEMENTATION (src/hardware/capabilities.rs):
  ```rust
  use async_trait::async_trait;
  use anyhow::Result;

  /// Capability to move a physical axis (stage, piezo, mirror)
  #[async_trait]
  pub trait Movable: Send + Sync {
      async fn move_abs(&self, position: f64) -> Result<()>;
      async fn move_rel(&self, distance: f64) -> Result<()>;
      async fn position(&self) -> Result<f64>;
      async fn wait_settled(&self) -> Result<()>;
  }

  /// Capability to trigger an acquisition event
  #[async_trait]
  pub trait Triggerable: Send + Sync {
      async fn arm(&self) -> Result<()>;
      async fn trigger(&self) -> Result<()>;
  }

  /// Capability to produce 2D frame data
  #[async_trait]
  pub trait FrameProducer: Send + Sync {
      async fn start_stream(&self) -> Result<()>;
      async fn stop_stream(&self) -> Result<()>;
      fn resolution(&self) -> (u32, u32);
  }
  ```

  MIGRATION FROM V3:
  - src/core_v3.rs traits → src/hardware/capabilities.rs
  - MotionController → Movable
  - Camera → FrameProducer + Triggerable
  - PowerMeter → NEW: Readable trait

  COMPILATION ERRORS FIXED:
  - elliptec.rs:492,505,510 (method parameter mismatches) - MIGRATE to Movable
  - esp300.rs:45 (unresolved MotionController) - USE Movable instead
  - pvcam.rs:874-1270 (missing Camera methods) - IMPLEMENT FrameProducer

  ACCEPTANCE:
  - src/hardware/capabilities.rs exists with 5+ capability traits
  - All traits use async_trait
  - All methods return anyhow::Result
  - Documentation includes usage examples

## Task C: Mock Driver Implementation
type: task
priority: P0
parent: bd-oq51.1
deps: bd-oq51.1.2
description: |
  Implement mock hardware drivers for testing without real devices.

  CREATE: src/hardware/mock.rs

  IMPLEMENTATIONS:
  ```rust
  use tokio::time::{sleep, Duration};
  use std::sync::Arc;
  use tokio::sync::RwLock;

  pub struct MockStage {
      position: Arc<RwLock<f64>>,
      name: String,
  }

  #[async_trait]
  impl Movable for MockStage {
      async fn move_abs(&self, target: f64) -> Result<()> {
          println!("[{}] Moving to {:.2}", self.name, target);
          sleep(Duration::from_millis(100)).await; // Simulate motion time
          *self.position.write().await = target;
          Ok(())
      }

      async fn position(&self) -> Result<f64> {
          Ok(*self.position.read().await)
      }

      // ... implement move_rel, wait_settled
  }

  pub struct MockCamera {
      resolution: (u32, u32),
      streaming: Arc<RwLock<bool>>,
  }

  #[async_trait]
  impl FrameProducer for MockCamera {
      async fn start_stream(&self) -> Result<()> {
          println!("[Camera] Starting stream at {}x{}", self.resolution.0, self.resolution.1);
          *self.streaming.write().await = true;
          Ok(())
      }

      fn resolution(&self) -> (u32, u32) {
          self.resolution
      }

      // ... implement stop_stream
  }
  ```

  TESTS:
  - Create tests/mock_hardware.rs
  - Test async move_abs completes
  - Test position() returns correct value
  - Test camera start/stop stream

  ACCEPTANCE:
  - src/hardware/mock.rs compiles
  - MockStage implements Movable
  - MockCamera implements FrameProducer + Triggerable
  - All tests pass (cargo test mock_hardware)
  - Mocks use tokio::time::sleep (not blocking sleep)
