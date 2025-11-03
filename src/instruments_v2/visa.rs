//! VISA Instrument V2 Module
//!
//! Generic VISA instrument implementation for V2 architecture.
//! Feature-gated with `instrument_visa`.

#[cfg(feature = "instrument_visa")]
mod visa_instrument_v2;

#[cfg(feature = "instrument_visa")]
pub use visa_instrument_v2::VisaInstrumentV2;

// Stub implementation when feature is disabled
#[cfg(not(feature = "instrument_visa"))]
pub struct VisaInstrumentV2;

#[cfg(not(feature = "instrument_visa"))]
impl VisaInstrumentV2 {
    pub fn new(_id: String, _resource: String) -> Self {
        panic!("instrument_visa feature not enabled. Rebuild with --features instrument_visa");
    }

    pub fn with_capacity(_id: String, _resource: String, _capacity: usize) -> Self {
        panic!("instrument_visa feature not enabled. Rebuild with --features instrument_visa");
    }

    pub fn with_streaming(self, _enabled: bool, _command: String, _rate_hz: f64) -> Self {
        panic!("instrument_visa feature not enabled. Rebuild with --features instrument_visa");
    }

    pub async fn send_command(&self, _command: &str) -> anyhow::Result<String> {
        anyhow::bail!("instrument_visa feature not enabled. Rebuild with --features instrument_visa")
    }

    pub async fn send_write(&self, _command: &str) -> anyhow::Result<()> {
        anyhow::bail!("instrument_visa feature not enabled. Rebuild with --features instrument_visa")
    }

    pub fn get_identity(&self) -> Option<&str> {
        None
    }
}