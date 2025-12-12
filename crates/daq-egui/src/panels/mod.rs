//! UI panels for the DAQ control application.

mod connection;
mod devices;
mod scripts;
mod scans;
mod storage;
mod modules;

pub use connection::ConnectionPanel;
pub use devices::DevicesPanel;
pub use scripts::ScriptsPanel;
pub use scans::ScansPanel;
pub use storage::StoragePanel;
pub use modules::ModulesPanel;
