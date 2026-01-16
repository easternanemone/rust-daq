// TODO: Fix doc comment links
#![allow(rustdoc::broken_intra_doc_links)]
// TODO: Address these clippy lints in a dedicated refactoring pass
#![allow(clippy::mixed_attributes_style)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::result_large_err)]
#![allow(clippy::single_match)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::vec_init_then_push)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::io_other_error)]

pub mod grpc;
pub mod health;
#[cfg(feature = "modules")]
pub mod modules;
#[cfg(feature = "rerun_sink")]
pub mod rerun_sink;

#[cfg(feature = "server")]
pub use grpc::server::DaqServer;

// Re-export Rerun types for server configuration
#[cfg(feature = "rerun_sink")]
pub use rerun::{MemoryLimit, ServerOptions};
#[cfg(feature = "rerun_sink")]
pub use rerun_sink::{APP_ID, DEFAULT_RERUN_PORT, RerunSink};
