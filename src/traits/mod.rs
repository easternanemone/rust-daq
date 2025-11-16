//! V4 Meta-Instrument Traits
//!
//! Hardware-agnostic trait definitions following DynExp pattern.

pub mod power_meter;
pub use power_meter::{PowerMeasurement, PowerMeter, PowerUnit, Wavelength};
