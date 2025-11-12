//! Core types and traits for the Rust DAQ system.
//!
//! This crate provides foundational types used throughout the DAQ system,
//! including high-precision timestamping with NTP synchronization.

pub mod timestamp;

// Re-export commonly used types
pub use timestamp::{Timestamp, TimestampSource};
