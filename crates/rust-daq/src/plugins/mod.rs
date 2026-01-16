//! Plugin system for rust-daq modules.
//!
//! This module provides infrastructure for loading modules from various sources:
//!
//! - **Native plugins** (via daq-plugin-api): Compiled Rust plugins using abi_stable
//! - **Script plugins** (this module): Rhai and Python scripts that implement modules
//!
//! # Architecture
//!
//! ```text
//! ModuleRegistry (rust-daq/src/modules/)
//! ├── Built-in modules (PowerMonitor, etc.)
//! ├── Native plugins (daq-plugin-api) [requires native_plugins feature]
//! │   ├── FfiModuleWrapper - Adapts FFI to Module trait
//! │   └── PluginModuleFactory - Creates wrapped instances
//! └── Script plugins (this module)
//!     ├── ScriptPluginLoader - Discovery and loading
//!     └── ScriptModule - Script-based Module implementation
//! ```

// Script plugins (always available)
pub mod loader;
pub mod script_module;

pub use loader::{ScriptLanguage, ScriptModuleInfo, ScriptPluginLoader};
pub use script_module::ScriptModule;

// Native plugins (FFI bridge) - requires native_plugins feature
#[cfg(feature = "native_plugins")]
mod native_plugins;

#[cfg(feature = "native_plugins")]
pub use native_plugins::{FfiModuleWrapper, PluginModuleFactory};

#[cfg(feature = "native_plugins")]
pub use daq_plugin_api::{LoadedPlugin, PluginManager};
