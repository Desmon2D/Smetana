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
        // depth = wall thickness (automatic, not stored)
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
    /// Create a default door (2100 x 900 mm).
    pub fn default_door() -> Self {
        OpeningKind::Door {
            height: 2100.0,
            width: 900.0,
        }
    }

    /// Create a default window (1400 x 1200 mm, sill 900 mm, reveal 250 mm).
    pub fn default_window() -> Self {
        OpeningKind::Window {
            height: 1400.0,
            width: 1200.0,
            sill_height: 900.0,
            reveal_width: 250.0,
        }
    }

    /// Opening width in mm.
    pub fn width(&self) -> f64 {
        match self {
            OpeningKind::Door { width, .. } => *width,
            OpeningKind::Window { width, .. } => *width,
        }
    }

    /// Opening height in mm.
    pub fn height(&self) -> f64 {
        match self {
            OpeningKind::Door { height, .. } => *height,
            OpeningKind::Window { height, .. } => *height,
        }
    }

    /// Returns the target object type for service assignment filtering.
    pub fn target_type(&self) -> crate::model::price::TargetObjectType {
        match self {
            OpeningKind::Door { .. } => crate::model::price::TargetObjectType::Door,
            OpeningKind::Window { .. } => crate::model::price::TargetObjectType::Window,
        }
    }
}

/// An opening (door or window) placed on a wall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    pub kind: OpeningKind,
    /// ID of the wall this opening is attached to. None = not attached (validation error).
    pub wall_id: Option<Uuid>,
    /// Offset from wall start to the center of the opening (mm).
    pub offset_along_wall: f64,
}

impl Opening {
    /// Create a new opening with the given kind, attached to a wall at the given offset.
    pub fn new(kind: OpeningKind, wall_id: Option<Uuid>, offset_along_wall: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            wall_id,
            offset_along_wall,
        }
    }

    /// Create a default door attached to a wall.
    pub fn new_door(wall_id: Uuid, offset_along_wall: f64) -> Self {
        Self::new(OpeningKind::default_door(), Some(wall_id), offset_along_wall)
    }

    /// Create a default window attached to a wall.
    pub fn new_window(wall_id: Uuid, offset_along_wall: f64) -> Self {
        Self::new(OpeningKind::default_window(), Some(wall_id), offset_along_wall)
    }
}
