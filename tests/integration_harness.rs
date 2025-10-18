//! Integration test harness for multi-instrument scenarios
//!
//! This test suite validates the DAQ system's ability to handle:
//! - Concurrent instrument spawning (10-20 instruments)
//! - Session save/load reliability (100 iterations)
//! - High-frequency command handling (1000 cmd/sec)
//! - Data flow integrity with multiple instruments
//!
//! See tests/README.md for detailed documentation.

mod integration;
