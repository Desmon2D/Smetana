use serde::{Deserialize, Serialize};
use uuid::Uuid;

use glam::DVec2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: Uuid,
    pub text: String,
    pub position: DVec2,
    /// Display font size in points (default 14.0)
    pub font_size: f64,
    /// Rotation in radians (default 0.0)
    pub rotation: f64,
}

impl Label {
    pub fn new(text: String, position: DVec2) -> Self {
        Self {
            id: Uuid::new_v4(),
            text,
            position,
            font_size: 14.0,
            rotation: 0.0,
        }
    }
}
