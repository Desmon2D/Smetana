use uuid::Uuid;
use crate::model::{Point2D, Wall, WallSide};

/// Screen-space radius (pixels) for snapping to existing wall vertices.
const VERTEX_SNAP_SCREEN_PX: f64 = 15.0;

/// What the cursor snapped to.
#[derive(Debug, Clone)]
pub enum SnapType {
    /// No snapping (Shift held — free drawing).
    None,
    /// Snapped to the nearest grid intersection.
    Grid,
    /// Snapped to an existing wall endpoint.
    Vertex,
    /// Snapped to a wall side edge (T-junction attachment point).
    WallEdge {
        wall_id: Uuid,
        side: WallSide,
        t: f64,
    },
}

/// Result of a snap operation: the snapped world position and what it snapped to.
#[derive(Debug, Clone)]
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

    // Wall edge snap: check proximity to wall side edges (for T-junctions)
    let mut closest_edge_dist = f64::MAX;
    let mut closest_edge: Option<(Uuid, WallSide, f64, Point2D)> = None;

    for wall in walls {
        let dx = wall.end.x - wall.start.x;
        let dy = wall.end.y - wall.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.0 {
            continue;
        }
        let half_t = wall.thickness / 2.0;
        // Left normal: (-dy/len, dx/len) * half_thickness
        let lnx = -dy / len * half_t;
        let lny = dx / len * half_t;

        for (side, sign) in [(WallSide::Left, 1.0), (WallSide::Right, -1.0)] {
            let edge_start = Point2D::new(wall.start.x + lnx * sign, wall.start.y + lny * sign);
            let edge_end = Point2D::new(wall.end.x + lnx * sign, wall.end.y + lny * sign);
            let (t, proj) = world_pos.project_onto_segment(edge_start, edge_end);
            // Only snap to interior of edge (not endpoints)
            if t > 0.01 && t < 0.99 {
                let dist = world_pos.distance_to(proj);
                if dist < snap_radius_world && dist < closest_edge_dist {
                    closest_edge_dist = dist;
                    closest_edge = Some((wall.id, side, t, proj));
                }
            }
        }
    }

    if let Some((wall_id, side, t, pos)) = closest_edge {
        return SnapResult {
            position: pos,
            snap_type: SnapType::WallEdge { wall_id, side, t },
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
