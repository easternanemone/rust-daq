//! Session management for the DAQ application.
//!
//! This module provides functionality for saving and loading the application's state.
//! A "session" captures the key aspects of the current setup, allowing a user to
//! restore it later. This is useful for quickly returning to a specific configuration
//! of instruments and settings.
//!
//! ## Session State
//!
//! The `Session` struct encapsulates the state that is saved, which includes:
//!
//! - **`active_instruments`**: A set of IDs for the instruments that are currently running.
//! - **`storage_settings`**: The current configuration for data storage, such as the
//!   default path and file format.
//! - **`gui_state`**: Information about the state of the GUI, such as the data currently
//!   displayed in plots. This allows the visual state to be restored as well.
//!
//! ## Functionality
//!
//! - **`save_session`**: Serializes a `Session` object into a JSON file at a specified path.
//! - **`load_session`**: Deserializes a `Session` object from a JSON file.
//!
//! Session construction and application is handled by `DaqManagerActor` which has direct
//! access to instrument state without needing blocking operations.
//!
//! This feature allows for greater experiment consistency and convenience, as complex
//! setups do not need to be manually reconfigured each time the application is started.

use crate::config::StorageSettings;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use std::collections::VecDeque;

/// Represents the state of the application to be saved in a session.
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub active_instruments: HashSet<String>,
    pub storage_settings: StorageSettings,
    pub gui_state: GuiState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuiState {
    pub plot_data: VecDeque<[f64; 2]>,
}

/// Saves the current session to a file.
pub fn save_session(session: &Session, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(session)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads a session from a file.
pub fn load_session(path: &Path) -> Result<Session> {
    let json = fs::read_to_string(path)?;
    let session = serde_json::from_str(&json)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_session() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_session.json");

        let mut active_instruments = HashSet::new();
        active_instruments.insert("mock_instrument".to_string());
        active_instruments.insert("another_instrument".to_string());

        let storage_settings = StorageSettings {
            default_path: "test/path".to_string(),
            default_format: "json".to_string(),
        };

        let gui_state = GuiState {
            plot_data: vec![[0.0, 1.0], [1.0, 2.0]].into(),
        };

        let session_to_save = Session {
            active_instruments: active_instruments.clone(),
            storage_settings: storage_settings.clone(),
            gui_state: gui_state.clone(),
        };

        // Save the session
        let save_result = save_session(&session_to_save, &file_path);
        assert!(save_result.is_ok());

        // Load the session
        let loaded_session = load_session(&file_path).unwrap();

        // Verify the contents
        assert_eq!(
            session_to_save.active_instruments,
            loaded_session.active_instruments
        );
        assert_eq!(
            session_to_save.storage_settings.default_path,
            loaded_session.storage_settings.default_path
        );
        assert_eq!(
            session_to_save.storage_settings.default_format,
            loaded_session.storage_settings.default_format
        );
        assert_eq!(
            session_to_save.gui_state.plot_data,
            loaded_session.gui_state.plot_data
        );
    }
}
