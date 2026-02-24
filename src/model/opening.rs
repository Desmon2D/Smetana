use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The kind of opening: door or window, with type-specific dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpeningKind {
    Door {
        /// Door height (mm)
        height: f64,
        /// Door width (mm)
        width: f64,
    },
    Window {
        /// Window height (mm)
        height: f64,
        /// Window width (mm)
        width: f64,
        /// Height from floor to window sill (mm)
        sill_height: f64,
        /// Reveal width (mm)
        reveal_width: f64,
    },
}

impl OpeningKind {
    /// Opening width in mm.
    #[allow(dead_code)]
    pub fn width(&self) -> f64 {
        match self {
            OpeningKind::Door { width, .. } => *width,
            OpeningKind::Window { width, .. } => *width,
        }
    }

    /// Opening height in mm.
    #[allow(dead_code)]
    pub fn height(&self) -> f64 {
        match self {
            OpeningKind::Door { height, .. } => *height,
            OpeningKind::Window { height, .. } => *height,
        }
    }
}

/// An opening (door or window) defined by a polygon of points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    /// Polygon vertices (point IDs) defining the opening footprint.
    pub points: Vec<Uuid>,
    pub kind: OpeningKind,
}

impl Opening {
    pub fn new(points: Vec<Uuid>, kind: OpeningKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            points,
            kind,
        }
    }
}
