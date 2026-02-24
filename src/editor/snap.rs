use glam::DVec2;
use uuid::Uuid;
use crate::model::{Wall, WallSide, project_onto_segment};

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
    pub position: DVec2,
    pub snap_type: SnapType,
}

impl SnapResult {
    pub fn wall_edge_junction(&self) -> Option<(Uuid, WallSide, f64)> {
        match &self.snap_type {
            SnapType::WallEdge { wall_id, side, t } => Some((*wall_id, *side, *t)),
            _ => None,
        }
    }
}

/// Compute the snapped position for a world-space cursor coordinate.
///
/// Priority: vertex snap > grid snap. Shift disables all snapping.
pub fn snap(
    world_pos: DVec2,
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
            let dist = world_pos.distance(endpoint);
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
    let mut closest_edge: Option<(Uuid, WallSide, f64, DVec2)> = None;

    for wall in walls {
        let d = wall.end - wall.start;
        let len = d.length();
        if len < 1.0 {
            continue;
        }
        let half_t = wall.thickness / 2.0;
        let normal = DVec2::new(-d.y / len, d.x / len) * half_t;

        for (side, sign) in [(WallSide::Left, 1.0), (WallSide::Right, -1.0)] {
            let edge_start = wall.start + normal * sign;
            let edge_end = wall.end + normal * sign;
            let (t, proj) = project_onto_segment(world_pos, edge_start, edge_end);
            // Only snap to interior of edge (not endpoints)
            if t > 0.01 && t < 0.99 {
                let dist = world_pos.distance(proj);
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
    let snapped = DVec2::new(
        (world_pos.x / grid_step).round() * grid_step,
        (world_pos.y / grid_step).round() * grid_step,
    );

    // Final pass: if a vertex is within 1mm of the grid-snapped position,
    // prefer the vertex to avoid phantom walls from float rounding.
    const VERTEX_EPSILON_MM: f64 = 1.0;
    for wall in walls {
        for endpoint in [wall.start, wall.end] {
            if snapped.distance(endpoint) < VERTEX_EPSILON_MM {
                return SnapResult {
                    position: endpoint,
                    snap_type: SnapType::Vertex,
                };
            }
        }
    }

    SnapResult {
        position: snapped,
        snap_type: SnapType::Grid,
    }
}
