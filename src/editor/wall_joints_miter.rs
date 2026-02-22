use std::collections::HashMap;

use eframe::egui;
use uuid::Uuid;

use super::wall_joints::{HubPolygon, JointVertices, WallAtJunction, MAX_MITER_RATIO};

/// Two-wall miter joint.
pub(super) fn compute_two_wall_miter(
    waj_list: &[WallAtJunction],
    joints: &mut HashMap<(Uuid, bool), JointVertices>,
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

    // Intersect a.left line with b.right line (they face each other between the walls).
    // And a.right line with b.left line.
    let miter_lr = line_line_intersection(
        a.left, a.dir, b.right, b.dir,
    );
    let miter_rl = line_line_intersection(
        a.right, a.dir, b.left, b.dir,
    );

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

    // Walk walls in angular order. Between consecutive walls i and i+1,
    // the edge facing the gap is: wall_i.right, wall_{i+1}.left.
    // We intersect the right edge line of wall_i with the left edge line of wall_{i+1}.
    // The intersection (or midpoint fallback) becomes a hub vertex and also
    // the adjusted joint vertex for both walls.

    for i in 0..n {
        let next = (i + 1) % n;
        let wa = &waj_list[i];
        let wb = &waj_list[next];

        // Between wa (right side) and wb (left side).
        let miter = line_line_intersection(wa.right, wa.dir, wb.left, wb.dir);

        let max_half = wa.half_thick.max(wb.half_thick);
        let junction_approx = egui::pos2(
            (wa.right.x + wb.left.x) / 2.0,
            (wa.right.y + wb.left.y) / 2.0,
        );
        let max_dist = max_half * MAX_MITER_RATIO;

        let pt = match miter {
            Some(p) if (p - junction_approx).length() < max_dist => p,
            _ => junction_approx,
        };

        hub_vertices.push(pt);
    }

    // Set joint vertices: wall i gets right = hub[i], left = hub[i-1].
    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let wa = &waj_list[i];
        joints.insert(
            (wa.wall_id, wa.is_end),
            JointVertices {
                left: hub_vertices[prev],
                right: hub_vertices[i],
            },
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
