use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Point2D;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: Uuid,
    pub text: String,
    pub position: Point2D,
    /// Display font size in points (default 14.0)
    pub font_size: f64,
    /// Rotation in radians (default 0.0)
    pub rotation: f64,
}

impl Label {
    pub fn new(text: String, position: Point2D) -> Self {
        Self {
            id: Uuid::new_v4(),
            text,
            position,
            font_size: 14.0,
            rotation: 0.0,
        }
    }
}
