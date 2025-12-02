//! Service layer for gRPC communication
//!
//! This module contains the gRPC client and related service abstractions.

mod client;
pub mod state_sync;

pub use client::DaqClient;
