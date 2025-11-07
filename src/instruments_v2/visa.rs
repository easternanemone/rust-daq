//! VISA Instrument V2 Module
//!
//! Generic VISA instrument implementation for V2 architecture.
//! Feature previously gated behind `instrument_visa`; support has now been removed.

/// Stub implementation preserved so existing code continues to compile.
pub struct VisaInstrumentV2;

impl VisaInstrumentV2 {
    pub fn new(_id: String, _resource: String) -> Self {
        panic!("VISA instrument support has been deprecated in rust_daq");
    }

    pub fn with_capacity(_id: String, _resource: String, _capacity: usize) -> Self {
        panic!("VISA instrument support has been deprecated in rust_daq");
    }

    pub fn with_streaming(self, _enabled: bool, _command: String, _rate_hz: f64) -> Self {
        panic!("VISA instrument support has been deprecated in rust_daq");
    }

    pub async fn send_command(&self, _command: &str) -> anyhow::Result<String> {
        anyhow::bail!("VISA instrument support has been deprecated in rust_daq")
    }

    pub async fn send_write(&self, _command: &str) -> anyhow::Result<()> {
        anyhow::bail!("VISA instrument support has been deprecated in rust_daq")
    }

    pub fn get_identity(&self) -> Option<&str> {
        None
    }
}
