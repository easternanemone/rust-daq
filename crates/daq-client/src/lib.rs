//! gRPC client library for rust-daq daemon.
//!
//! This crate provides a high-level client for communicating with the rust-daq
//! daemon over gRPC. It is UI-agnostic and can be used by CLI tools, test
//! harnesses, and alternative frontends.

pub mod connection;
pub mod error;

// These modules will be added in subsequent subtasks:
// pub mod client;      // bd-xrnw.3
// pub mod reconnect;   // bd-xrnw.4

pub use connection::{
    normalize_url, resolve_address, AddressError, AddressSource, DaemonAddress, DEFAULT_DAEMON_URL,
    DEFAULT_GRPC_PORT, STORAGE_KEY_DAEMON_ADDR,
};
pub use error::{ClientError, Result};
