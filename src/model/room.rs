use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Which side of a wall faces the room interior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallSide {
    /// The left side when looking from wall.start to wall.end
    Left,
    /// The right side when looking from wall.start to wall.end
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    /// Ordered list of wall IDs forming a closed contour
    pub wall_ids: Vec<Uuid>,
    /// For each wall — which side faces the room interior
    pub wall_sides: Vec<WallSide>,
}

impl Room {
    pub fn new(name: String, wall_ids: Vec<Uuid>, wall_sides: Vec<WallSide>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            wall_ids,
            wall_sides,
        }
    }
}
