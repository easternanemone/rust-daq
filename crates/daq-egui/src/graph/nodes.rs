//! Experiment node definitions for the graph editor.

use serde::{Deserialize, Serialize};

/// Node types for experiment workflows.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExperimentNode {
    /// 1D or 2D parameter scan
    Scan {
        actuator: String, // Device ID for movable
        start: f64,
        stop: f64,
        points: u32,
    },
    /// Single acquisition from detector
    Acquire {
        detector: String, // Device ID for readable/camera
        duration_ms: f64,
    },
    /// Move actuator to position
    Move {
        device: String,
        position: f64,
    },
    /// Wait/delay step
    Wait { duration_ms: f64 },
    /// Loop control node
    Loop { iterations: u32 },
}

impl ExperimentNode {
    /// Get human-readable name for this node type.
    pub fn node_name(&self) -> &'static str {
        match self {
            ExperimentNode::Scan { .. } => "Scan",
            ExperimentNode::Acquire { .. } => "Acquire",
            ExperimentNode::Move { .. } => "Move",
            ExperimentNode::Wait { .. } => "Wait",
            ExperimentNode::Loop { .. } => "Loop",
        }
    }

    /// Create a default Scan node with sensible defaults.
    pub fn default_scan() -> Self {
        ExperimentNode::Scan {
            actuator: String::new(),
            start: 0.0,
            stop: 100.0,
            points: 10,
        }
    }

    /// Create a default Acquire node with sensible defaults.
    pub fn default_acquire() -> Self {
        ExperimentNode::Acquire {
            detector: String::new(),
            duration_ms: 100.0,
        }
    }

    /// Create a default Move node with sensible defaults.
    pub fn default_move() -> Self {
        ExperimentNode::Move {
            device: String::new(),
            position: 0.0,
        }
    }

    /// Create a default Wait node with sensible defaults.
    pub fn default_wait() -> Self {
        ExperimentNode::Wait {
            duration_ms: 1000.0,
        }
    }

    /// Create a default Loop node with sensible defaults.
    pub fn default_loop() -> Self {
        ExperimentNode::Loop { iterations: 10 }
    }
}
