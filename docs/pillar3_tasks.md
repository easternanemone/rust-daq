# Pillar 3: Unify Data Transport - Task Breakdown

## P3.1: Standardize on core_v3::Measurement Enum
type: task
priority: P0
parent: bd-kal8.3
description: |
  Make core_v3::Measurement the ONLY transport type in codebase.

  UPDATES NEEDED:
  - All instruments emit core_v3::Measurement
  - DataDistributor accepts core_v3::Measurement
  - GUI consumes core_v3::Measurement
  - NO Arrow in instrument layer

  ERRORS TO FIX:
  - All Measurement import mismatches
  - InstrumentState enum import mismatches

  ACCEPTANCE:
  - grep "Measurement" shows only core_v3::Measurement
  - Zero daq_core::Measurement references

## P3.2: Fix Trait Signature Type Mismatches
type: task
priority: P0
parent: bd-kal8.3
deps: bd-kal8.3.1
description: |
  Fix all trait method signature type mismatches.

  ERRORS TO FIX:
  - elliptec.rs:301 (type alias 1 vs 2 generics)
  - elliptec.rs:492,505,510 (method parameter counts)
  - elliptec.rs:554 (lifetime parameter mismatch)
  - mock_instrument.rs:371,383,395 (lifetime mismatches)
  - All PowerRange trait mismatches

  PATTERN:
  - Check trait definition in daq_core
  - Match signature exactly (parameters, lifetimes, return types)

  ACCEPTANCE:
  - All E0050, E0053, E0195 errors fixed
  - Trait implementations compile

## P3.3: Implement Arrow Batching in DataDistributor
type: task
priority: P0
parent: bd-kal8.3
deps: bd-kal8.3.1
description: |
  Add Arrow batching layer in DataDistributor for storage optimization.

  DESIGN:
  - DataDistributor receives V3 Measurement enums
  - Accumulates measurements in buffer
  - Converts batch to Arrow RecordBatch
  - Sends RecordBatch to storage actors

  ACCEPTANCE:
  - Measurement enum → Arrow conversion happens ONLY in DataDistributor
  - Storage actors receive Arrow RecordBatch
  - GUI still receives V3 Measurement (no conversion)
  - Batch size configurable (e.g., 1000 measurements)

## P3.4: Remove Arrow from Instrument Layer
type: task
priority: P0
parent: bd-kal8.3
deps: bd-kal8.3.3
description: |
  Ensure NO instrument directly produces Arrow data.

  AUDIT:
  - Check all instruments in src/instruments_v2/
  - Remove any Arrow RecordBatch generation
  - Replace with V3 Measurement enum

  ACCEPTANCE:
  - grep "RecordBatch" in src/instruments_v2/ returns 0
  - All instruments use V3 Measurement
  - Compilation errors < 20

## P3.5: Fix HDF5 Storage to Accept Arrow Batches
type: task
priority: P0
parent: bd-kal8.3
deps: bd-kal8.3.3
description: |
  Update HDF5 storage writer to consume Arrow RecordBatch from DataDistributor.

  ERRORS TO FIX:
  - hdf5_storage.rs:314 (unused mut)
  - hdf5_storage.rs:369 (unused msg variable)
  - hdf5_storage.rs:36,43,51,53 (dead code fields)

  IMPLEMENTATION:
  - Storage actor receives RecordBatch
  - Converts Arrow → HDF5 efficiently
  - Batch writes (not individual measurements)

  ACCEPTANCE:
  - HDF5 storage compiles
  - Accepts Arrow batches from DataDistributor
  - Write throughput > 10k measurements/sec
