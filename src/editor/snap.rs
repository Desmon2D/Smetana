use glam::DVec2;
use uuid::Uuid;

use crate::model::Point;

/// Result of a snap operation.
pub struct SnapResult {
    /// Snapped world position (mm).
    pub position: DVec2,
    /// If snapped to an existing point, its ID.
    pub snapped_point: Option<Uuid>,
}

/// Screen-space radius (pixels) within which we snap to an existing point.
const POINT_SNAP_RADIUS: f64 = 15.0;

/// Find the nearest existing point within the screen-space snap radius.
/// Returns the point ID if found.
pub fn snap_to_point(world_pos: DVec2, points: &[Point], zoom: f32) -> Option<Uuid> {
    let threshold = POINT_SNAP_RADIUS / zoom as f64; // convert screen px to world mm
    let mut best: Option<(Uuid, f64)> = None;
    for p in points {
        let dist = p.position.distance(world_pos);
        if dist < threshold && (best.is_none() || dist < best.unwrap().1) {
            best = Some((p.id, dist));
        }
    }
    best.map(|(id, _)| id)
}

/// Snap a world position to the nearest grid intersection.
pub fn snap_to_grid(world_pos: DVec2, grid_step: f64) -> DVec2 {
    DVec2::new(
        (world_pos.x / grid_step).round() * grid_step,
        (world_pos.y / grid_step).round() * grid_step,
    )
}

/// Combined snap: try point snap first, then grid snap.
/// If `snap_enabled` is false, returns the raw world position (no snapping).
pub fn snap(
    world_pos: DVec2,
    points: &[Point],
    grid_step: f64,
    zoom: f32,
    snap_enabled: bool,
) -> SnapResult {
    if !snap_enabled {
        return SnapResult {
            position: world_pos,
            snapped_point: None,
        };
    }

    // 1. Try snapping to an existing point
    if let Some(id) = snap_to_point(world_pos, points, zoom) {
        let pos = points.iter().find(|p| p.id == id).unwrap().position;
        return SnapResult {
            position: pos,
            snapped_point: Some(id),
        };
    }

    // 2. Fall back to grid snap
    let grid_pos = snap_to_grid(world_pos, grid_step);
    SnapResult {
        position: grid_pos,
        snapped_point: None,
    }
}
