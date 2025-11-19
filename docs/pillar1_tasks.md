# Pillar 1: Delete V1/V2 Legacy - Task Breakdown

## P1.1: Delete V2 App Actor and Core Files
type: task
priority: P0
parent: bd-kal8.1
description: |
  Delete the V2 monolithic actor architecture entirely.

  FILES TO DELETE:
  - src/app_actor.rs (V2 DaqApp actor)
  - src/adapters/v2_adapter.rs (compatibility wrapper)

  ERRORS FIXED:
  - v2_adapter.rs:575-604 (missing trait methods)
  - v2_adapter.rs:579 (InstrumentState type mismatch)

  ACCEPTANCE:
  - Files deleted and removed from mod.rs
  - Compilation errors reduced by ~15

## P1.2: Delete V1 Core Trait Definitions
type: task
priority: P0
parent: bd-kal8.1
deps: bd-kal8.1.1
description: |
  Delete V1 Instrument<M> trait and DataPoint type from src/core.rs.
  Keep ONLY V3 definitions in this file or move to core_v3.rs.

  DELETE FROM src/core.rs:
  - trait Instrument<M: Measurement>
  - struct DataPoint (replaced by V3 Measurement enum)
  - trait Measurement
  - All V1 adapter traits

  ACCEPTANCE:
  - src/core.rs contains only V3 OR is deleted entirely
  - core_v3.rs is the canonical core

## P1.3: Delete V2 Registry Compatibility Layer
type: task
priority: P0
parent: bd-kal8.1
deps: bd-kal8.1.1
description: |
  Delete registry_v2.rs and all V2 instrument registry code.

  FILES TO DELETE:
  - src/instrument/registry_v2.rs

  ERRORS FIXED:
  - registry_v2.rs:90-124 (missing trait methods)
  - registry_v2.rs:99 (InstrumentState mismatch)

  ACCEPTANCE:
  - Only InstrumentManagerV3 exists
  - Zero V2 registry references

## P1.4: Fix V3 Import Consolidation After Deletion
type: task
priority: P0
parent: bd-kal8.1
deps: bd-kal8.1.1,bd-kal8.1.2,bd-kal8.1.3
description: |
  After deleting V1/V2, consolidate all imports to use core_v3.

  UPDATES NEEDED:
  - All use daq_core::* → use crate::core_v3::*
  - All use core::* → use crate::core_v3::*
  - Update main.rs bootstrap to V3 only

  ACCEPTANCE:
  - grep "use.*core::" shows only core_v3
  - Zero V1/V2 import paths remain
  - Compilation errors < 50

## P1.5: Remove V1/V2 Test Files
type: task
priority: P0
parent: bd-kal8.1
deps: bd-kal8.1.4
description: |
  Delete all tests that reference V1/V2 architectures.

  DELETE:
  - tests/*v2*.rs
  - Any test using V1 Instrument<M> trait
  - app_actor test files

  ACCEPTANCE:
  - All remaining tests use V3 APIs
  - cargo test compiles (may fail, but compiles)
