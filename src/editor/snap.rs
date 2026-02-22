use crate::model::{Point2D, Wall};

/// Screen-space radius (pixels) for snapping to existing wall vertices.
const VERTEX_SNAP_SCREEN_PX: f64 = 15.0;

/// What the cursor snapped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapType {
    /// No snapping (Shift held — free drawing).
    None,
    /// Snapped to the nearest grid intersection.
    Grid,
    /// Snapped to an existing wall endpoint.
    Vertex,
}

/// Result of a snap operation: the snapped world position and what it snapped to.
#[derive(Debug, Clone, Copy)]
pub struct SnapResult {
    pub position: Point2D,
    pub snap_type: SnapType,
}

/// Compute the snapped position for a world-space cursor coordinate.
///
/// Priority: vertex snap > grid snap. Shift disables all snapping.
pub fn snap(
    world_pos: Point2D,
    grid_step: f64,
    zoom: f32,
    walls: &[Wall],
    shift_held: bool,
) -> SnapResult {
    if shift_held {
        return SnapResult {
            position: world_pos,
            snap_type: SnapType::None,
        };
    }

    // Vertex snap: find the closest wall endpoint within the screen-space radius
    let snap_radius_world = VERTEX_SNAP_SCREEN_PX / zoom as f64;
    let mut closest_dist = f64::MAX;
    let mut closest_vertex = None;

    for wall in walls {
        for endpoint in [wall.start, wall.end] {
            let dist = world_pos.distance_to(endpoint);
            if dist < snap_radius_world && dist < closest_dist {
                closest_dist = dist;
                closest_vertex = Some(endpoint);
            }
        }
    }

    if let Some(vertex) = closest_vertex {
        return SnapResult {
            position: vertex,
            snap_type: SnapType::Vertex,
        };
    }

    // Grid snap: round to nearest grid intersection
    let snapped = Point2D {
        x: (world_pos.x / grid_step).round() * grid_step,
        y: (world_pos.y / grid_step).round() * grid_step,
    };
    SnapResult {
        position: snapped,
        snap_type: SnapType::Grid,
    }
}
