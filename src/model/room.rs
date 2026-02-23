use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::wall::Point2D;

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
    /// Segment endpoints for each wall entry: (from, to) in room traversal
    /// order. These are the actual portion of the wall centerline used in
    /// the room boundary, which may differ from wall.start/wall.end when
    /// T-junctions split walls into segments.
    #[serde(default)]
    pub wall_segments: Vec<(Point2D, Point2D)>,
}

impl Room {
    pub fn new(
        name: String,
        wall_ids: Vec<Uuid>,
        wall_sides: Vec<WallSide>,
        wall_segments: Vec<(Point2D, Point2D)>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            wall_ids,
            wall_sides,
            wall_segments,
        }
    }
}
