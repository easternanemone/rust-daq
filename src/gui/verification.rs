//! GUI Verification Framework for Agent Workflows
//!
//! This module provides tools for automated verification of GUI state,
//! similar to Playwright for web applications but designed for egui desktop apps.
//!
//! ## Usage in Agent Workflows
//!
//! Agents can use this framework to:
//! - Verify UI elements are visible and correct
//! - Capture screenshots for visual verification
//! - Check application state programmatically
//!
//! ## Example: bd + Jules Integration
//!
//! When working on a GUI-related bd issue, agents can:
//! 1. Run the application
//! 2. Use verification scripts to check UI changes
//! 3. Capture screenshots as proof of completion
//! 4. Attach screenshots to PR for review
//!
//! See `jules-scratch/verification/` for example verification scripts.

use std::path::PathBuf;

/// Verification command that can be sent to the GUI
#[derive(Debug, Clone)]
pub enum VerificationCommand {
    /// Take a screenshot and save to specified path
    Screenshot(PathBuf),
    /// Verify that a UI element with the given text exists
    VerifyElementExists(String),
    /// Verify application is responsive
    VerifyResponsive,
}

/// Result of a verification operation
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub success: bool,
    pub message: String,
    pub screenshot_path: Option<PathBuf>,
}

impl VerificationResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            screenshot_path: None,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            screenshot_path: None,
        }
    }

    pub fn with_screenshot(mut self, path: PathBuf) -> Self {
        self.screenshot_path = Some(path);
        self
    }
}

/// Helper struct for verification operations
pub struct VerificationHelper {
    screenshot_dir: PathBuf,
}

impl VerificationHelper {
    /// Create a new verification helper
    ///
    /// # Arguments
    /// * `screenshot_dir` - Directory where screenshots will be saved
    pub fn new(screenshot_dir: impl Into<PathBuf>) -> Self {
        Self {
            screenshot_dir: screenshot_dir.into(),
        }
    }

    /// Generate a timestamped screenshot path
    pub fn screenshot_path(&self, name: &str) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        self.screenshot_dir
            .join(format!("{}_{}.png", name, timestamp))
    }

    /// Generate a named screenshot path (for agent verification)
    pub fn named_screenshot_path(&self, name: &str) -> PathBuf {
        self.screenshot_dir.join(format!("{}.png", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result() {
        let result = VerificationResult::success("Test passed");
        assert!(result.success);
        assert_eq!(result.message, "Test passed");
        assert!(result.screenshot_path.is_none());
    }

    #[test]
    fn test_verification_helper() {
        let helper = VerificationHelper::new("screenshots");
        let path = helper.named_screenshot_path("test_ui");
        assert_eq!(path, PathBuf::from("screenshots/test_ui.png"));
    }
}
