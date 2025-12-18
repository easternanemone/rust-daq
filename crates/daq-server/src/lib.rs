pub mod grpc;
pub mod health;
pub mod rerun_sink;
#[cfg(feature = "modules")]
pub mod modules;

#[cfg(feature = "server")]
pub use grpc::server::DaqServer;

// Re-export Rerun types for server configuration
pub use rerun::{MemoryLimit, ServerOptions};
pub use rerun_sink::{RerunSink, APP_ID, DEFAULT_RERUN_PORT};
