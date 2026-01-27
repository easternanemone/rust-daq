//! gRPC client library for rust-daq daemon.
//!
//! This crate provides a high-level client for communicating with the rust-daq
//! daemon over gRPC. It is UI-agnostic and can be used by CLI tools, test
//! harnesses, and alternative frontends.

pub mod error;

// These modules will be added in subsequent subtasks:
// pub mod client;      // bd-xrnw.3
// pub mod connection;  // bd-xrnw.2
// pub mod reconnect;   // bd-xrnw.4

pub use error::{ClientError, Result};
