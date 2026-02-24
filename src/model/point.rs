use glam::DVec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub id: Uuid,
    /// Canvas position in mm (world coordinates)
    pub position: DVec2,
    /// Ceiling height at this point in mm
    pub height: f64,
}

impl Point {
    pub fn new(position: DVec2, height: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            height,
        }
    }
}
