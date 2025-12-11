//! Protocol buffer definitions and conversions for rust-daq.
//!
//! This crate contains:
//! - Generated protobuf types from `proto/daq.proto`
//! - Health check service from `proto/health.proto`
//! - Conversion traits between proto types and domain types in `daq-core`
//!
//! # Architecture
//!
//! The proto types are kept separate from domain types to:
//! - Avoid transport-layer coupling in domain code
//! - Enable modules to work without networking features
//! - Provide clear boundaries for type conversions

#![allow(missing_docs)] // Generated code doesn't have docs

pub mod convert;

/// Generated DAQ protocol buffer types.
pub mod daq {
    tonic::include_proto!("daq");
}

/// Generated health check protocol buffer types.
pub mod health {
    tonic::include_proto!("grpc.health.v1");
}

// Re-export commonly used types at crate root
pub use daq::*;
