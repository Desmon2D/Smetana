use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A visual wall polygon on the canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    /// Polygon vertices (point IDs) defining the wall shape.
    pub points: Vec<Uuid>,
    /// Fill color (RGBA).
    pub color: [u8; 4],
}

impl Wall {
    pub fn new(points: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            points,
            color: [180, 180, 180, 255], // default gray
        }
    }
}
