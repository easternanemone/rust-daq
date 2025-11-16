//! V4 Hardware Adapters
//!
//! Low-level hardware communication abstractions (VISA, Serial, USB).

pub mod serial_adapter_v4;
pub use serial_adapter_v4::{SerialAdapterV4, SerialAdapterV4Builder};
