// TODO: Fix doc comment generic types (e.g., `Parameter<T>`) to use backticks
// and broken intra-doc links (e.g., `#[async_trait]`)
#![allow(rustdoc::invalid_html_tags)]
#![allow(rustdoc::broken_intra_doc_links)]

pub mod core;
// Data types (Frame, etc.)
pub mod data;
// Document model (Bluesky-style)
pub mod capabilities;
pub mod error;
pub mod error_recovery;
pub mod experiment;
pub mod health;
pub mod limits;
pub mod modules;
pub mod observable;
pub mod parameter;
pub mod pipeline;

// Driver factory and capability types for plugin architecture
pub mod driver;

// Serial port abstractions for driver crates (requires "serial" feature)
#[cfg(feature = "serial")]
pub mod serial;
