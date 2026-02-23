use std::collections::HashMap;

use eframe::egui;
use uuid::Uuid;

use super::wall_joints::{HubPolygon, JointVertices, WallAtJunction, MAX_MITER_RATIO};

/// Two-wall miter joint with hub polygon to cover internal outline artifacts.
pub(super) fn compute_two_wall_miter(
    waj_list: &[WallAtJunction],
    joints: &mut HashMap<(Uuid, bool), JointVertices>,
    hubs: &mut Vec<HubPolygon>,
    fill: egui::Color32,
) {
    let a = &waj_list[0];
    let b = &waj_list[1];

    // Junction point (average of the two from-points, which should be ~same).
    let junction = egui::pos2(
        (a.left.x + a.right.x + b.left.x + b.right.x) / 4.0,
        (a.left.y + a.right.y + b.left.y + b.right.y) / 4.0,
    );

    let max_half = a.half_thick.max(b.half_thick);
    let max_dist = max_half * MAX_MITER_RATIO;

    // Intersect a.left with b.right and a.right with b.left (cross-side = miter points).
    let miter_lr = line_line_intersection(a.left, a.dir, b.right, b.dir);
    let miter_rl = line_line_intersection(a.right, a.dir, b.left, b.dir);

    // Intersect same-side edges (a.left with b.left, a.right with b.right).
    let ll = line_line_intersection(a.left, a.dir, b.left, b.dir);
    let rr = line_line_intersection(a.right, a.dir, b.right, b.dir);

    // For wall A:
    let a_left = match miter_lr {
        Some(pt) if (pt - junction).length() < max_dist => pt,
        _ => a.left,
    };
    let a_right = match miter_rl {
        Some(pt) if (pt - junction).length() < max_dist => pt,
        _ => a.right,
    };

    // For wall B:
    let b_left = match miter_rl {
        Some(pt) if (pt - junction).length() < max_dist => pt,
        _ => b.left,
    };
    let b_right = match miter_lr {
        Some(pt) if (pt - junction).length() < max_dist => pt,
        _ => b.right,
    };

    joints.insert(
        (a.wall_id, a.is_end),
        JointVertices { left: a_left, right: a_right },
    );
    joints.insert(
        (b.wall_id, b.is_end),
        JointVertices { left: b_left, right: b_right },
    );

    // Build hub polygon from all 4 intersection points to cover the corner area.
    let candidates = [miter_lr, miter_rl, ll, rr];
    let mut hub_vertices: Vec<egui::Pos2> = candidates
        .iter()
        .filter_map(|opt| {
            opt.filter(|pt| (*pt - junction).length() < max_dist)
        })
        .collect();

    if hub_vertices.len() >= 3 {
        // Sort by angle from centroid to ensure correct polygon winding.
        let cx = hub_vertices.iter().map(|p| p.x).sum::<f32>() / hub_vertices.len() as f32;
        let cy = hub_vertices.iter().map(|p| p.y).sum::<f32>() / hub_vertices.len() as f32;
        hub_vertices.sort_by(|a, b| {
            let angle_a = (a.y - cy).atan2(a.x - cx);
            let angle_b = (b.y - cy).atan2(b.x - cx);
            angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        hubs.push(HubPolygon {
            vertices: hub_vertices,
            fill,
        });
    }
}

/// Three+ wall hub polygon.
pub(super) fn compute_hub_polygon(
    waj_list: &[WallAtJunction],
    joints: &mut HashMap<(Uuid, bool), JointVertices>,
    hubs: &mut Vec<HubPolygon>,
    fill: egui::Color32,
) {
    let n = waj_list.len();
    let mut hub_vertices: Vec<egui::Pos2> = Vec::new();

    // Walk walls in angular order (sorted by increasing atan2 angle =
    // clockwise on screen with Y-down). Between consecutive walls i and
    // i+1, the gap is bounded by wall_i's CW-facing edge and wall_{i+1}'s
    // CCW-facing edge. Which wall edge faces CW/CCW depends on is_end:
    //   is_end=false → CW-facing = left,  CCW-facing = right
    //   is_end=true  → CW-facing = right, CCW-facing = left

    for i in 0..n {
        let next = (i + 1) % n;
        let wa = &waj_list[i];
        let wb = &waj_list[next];

        let miter = line_line_intersection(wa.cw_edge(), wa.dir, wb.ccw_edge(), wb.dir);

        let max_half = wa.half_thick.max(wb.half_thick);
        let junction_approx = egui::pos2(
            (wa.cw_edge().x + wb.ccw_edge().x) / 2.0,
            (wa.cw_edge().y + wb.ccw_edge().y) / 2.0,
        );
        let max_dist = max_half * MAX_MITER_RATIO;

        let pt = match miter {
            Some(p) if (p - junction_approx).length() < max_dist => p,
            _ => junction_approx,
        };

        hub_vertices.push(pt);
    }

    // Set joint vertices: hub[i] is the CW-side miter point for wall i,
    // hub[prev] is the CCW-side miter point. Map back to left/right
    // based on is_end.
    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let wa = &waj_list[i];
        let (left, right) = if wa.is_end {
            // is_end: CW-facing = right, CCW-facing = left
            (hub_vertices[prev], hub_vertices[i])
        } else {
            // !is_end: CW-facing = left, CCW-facing = right
            (hub_vertices[i], hub_vertices[prev])
        };
        joints.insert(
            (wa.wall_id, wa.is_end),
            JointVertices { left, right },
        );
    }

    hubs.push(HubPolygon {
        vertices: hub_vertices,
        fill,
    });
}

/// Intersect two infinite lines: line through `p1` in direction `d1`,
/// and line through `p2` in direction `d2`.
/// Returns None if lines are parallel.
pub(super) fn line_line_intersection(
    p1: egui::Pos2,
    d1: egui::Vec2,
    p2: egui::Pos2,
    d2: egui::Vec2,
) -> Option<egui::Pos2> {
    let denom = d1.x * d2.y - d1.y * d2.x;
    if denom.abs() < 1e-6 {
        return None;
    }
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let t = (dx * d2.y - dy * d2.x) / denom;
    Some(egui::pos2(p1.x + t * d1.x, p1.y + t * d1.y))
}
