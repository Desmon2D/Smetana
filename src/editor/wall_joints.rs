use std::collections::HashMap;

use eframe::egui;
use glam::DVec2;
use uuid::Uuid;

use crate::editor::endpoint_merge::merge_endpoints;
use crate::model::Wall;

/// Merge epsilon for detecting shared endpoints (mm).
const MERGE_EPS: f64 = 5.0;

/// Maximum miter extension as a multiple of the thicker wall's half-thickness.
const MAX_MITER_RATIO: f64 = 3.0;

/// Computed joint vertices for one end of a wall (world coordinates, mm).
pub struct JointVertices {
    /// Adjusted left-side vertex (left = positive normal direction).
    pub left: DVec2,
    /// Adjusted right-side vertex (right = negative normal direction).
    pub right: DVec2,
}

/// A hub polygon at a wall junction, drawn on top to cover internal outline artifacts.
pub struct HubPolygon {
    pub vertices: Vec<DVec2>,
    pub fill: egui::Color32,
}

/// Per-wall direction info at a junction (precomputed, world coordinates).
struct WallAtJunction {
    wall_id: Uuid,
    is_end: bool,
    /// Outgoing angle from junction (radians).
    angle: f64,
    /// Half-thickness in mm.
    half_thick: f64,
    /// Left edge point at the junction (perpendicular offset, world coords).
    left: DVec2,
    /// Right edge point at the junction (perpendicular offset, world coords).
    right: DVec2,
    /// Outgoing unit direction from junction.
    dir: DVec2,
}

impl WallAtJunction {
    /// Edge facing the CW direction (increasing atan2 angle).
    /// - is_end=false: left
    /// - is_end=true: right
    fn cw_edge(&self) -> DVec2 {
        if self.is_end { self.right } else { self.left }
    }

    /// Edge facing the CCW direction.
    fn ccw_edge(&self) -> DVec2 {
        if self.is_end { self.left } else { self.right }
    }
}

/// Compute adjusted joint vertices for all wall endpoints and hub polygons.
/// All geometry is computed in world space (mm). The caller converts to
/// screen coordinates at render time.
pub fn compute_joints(
    walls: &[Wall],
) -> (HashMap<(Uuid, bool), JointVertices>, Vec<HubPolygon>) {
    let mut joints: HashMap<(Uuid, bool), JointVertices> = HashMap::new();
    let mut hubs: Vec<HubPolygon> = Vec::new();

    if walls.is_empty() {
        return (joints, hubs);
    }

    // Group wall endpoints by junction (merge within MERGE_EPS).
    let junction_groups = merge_endpoints(walls, MERGE_EPS);

    let wall_fill = egui::Color32::from_rgb(140, 140, 145);

    for (_jpos, members) in &junction_groups {
        if members.len() < 2 {
            continue; // Solo endpoint — no joint needed.
        }

        // Build WallAtJunction for each wall at this junction.
        let mut waj_list: Vec<WallAtJunction> = Vec::new();
        for &(wall_id, is_end) in members {
            let wall = match walls.iter().find(|w| w.id == wall_id) {
                Some(w) => w,
                None => continue,
            };

            let (from, to) = if is_end {
                (wall.end, wall.start) // outgoing direction = away from junction
            } else {
                (wall.start, wall.end)
            };

            // Outgoing direction from the junction:
            let out = to - from;
            let out_len = out.length();
            if out_len < 1e-6 {
                continue;
            }
            let dir = out / out_len;
            let angle = out.y.atan2(out.x);

            let half_thick = wall.thickness / 2.0;

            // Left normal matching the convention in draw_walls:
            // For is_end=false (junction at start): wall dir = outgoing dir
            //   left normal = (-dir.y, dir.x)
            // For is_end=true (junction at end): wall dir start→end = -outgoing dir
            //   left normal = (dir.y, -dir.x)
            let normal_left = if !is_end {
                DVec2::new(-dir.y, dir.x)
            } else {
                DVec2::new(dir.y, -dir.x)
            };

            let left = from + normal_left * half_thick;
            let right = from - normal_left * half_thick;

            waj_list.push(WallAtJunction {
                wall_id,
                is_end,
                angle,
                half_thick,
                left,
                right,
                dir,
            });
        }

        if waj_list.len() < 2 {
            continue;
        }

        // Sort by outgoing angle.
        waj_list.sort_by(|a, b| a.angle.partial_cmp(&b.angle).unwrap_or(std::cmp::Ordering::Equal));

        if waj_list.len() == 2 {
            compute_two_wall_miter(&waj_list, &mut joints, &mut hubs, wall_fill);
        } else {
            compute_hub_polygon(&waj_list, &mut joints, &mut hubs, wall_fill);
        }
    }

    // T-junction end trimming: when a wall connects to another wall's side,
    // trim its end face to be flush with the host wall's side surface.
    for host_wall in walls {
        for (side_data, sign) in [(&host_wall.left_side, 1.0_f64), (&host_wall.right_side, -1.0_f64)] {
            if side_data.junctions.is_empty() {
                continue;
            }

            let host_d = host_wall.end - host_wall.start;
            let host_len = host_d.length();
            if host_len < 1e-6 {
                continue;
            }

            // Host wall direction and left normal (looking start→end).
            let host_dir = host_d / host_len;
            let host_normal = DVec2::new(-host_dir.y, host_dir.x);
            let host_half_thick = host_wall.thickness / 2.0;

            // A point on the host wall's side edge line.
            let edge_point = host_wall.start + host_normal * host_half_thick * sign;

            for junction in &side_data.junctions {
                let conn_wall = match walls.iter().find(|w| w.id == junction.wall_id) {
                    Some(w) => w,
                    None => continue,
                };

                // Junction point on the host wall's side edge.
                let junc_pt = host_wall.start + host_d * junction.t
                    + host_normal * host_half_thick * sign;

                // Determine which end of the connecting wall is at the junction.
                let start_dist = conn_wall.start.distance(junc_pt);
                let end_dist = conn_wall.end.distance(junc_pt);
                let is_end = end_dist < start_dist;

                // Skip if already handled by endpoint junction grouping.
                if joints.contains_key(&(conn_wall.id, is_end)) {
                    continue;
                }

                // Connecting wall direction (start→end) and left normal.
                let conn_d = conn_wall.end - conn_wall.start;
                let conn_len = conn_d.length();
                if conn_len < 1e-6 {
                    continue;
                }

                let conn_dir = conn_d / conn_len;
                let conn_normal = DVec2::new(-conn_dir.y, conn_dir.x);
                let conn_half_thick = conn_wall.thickness / 2.0;

                // Connecting wall's left/right edge lines (parallel to centerline).
                let left_edge_pt = conn_wall.start + conn_normal * conn_half_thick;
                let right_edge_pt = conn_wall.start - conn_normal * conn_half_thick;

                // Intersect each edge with the host wall's side edge line.
                let left_int = line_line_intersection(left_edge_pt, conn_dir, edge_point, host_dir);
                let right_int =
                    line_line_intersection(right_edge_pt, conn_dir, edge_point, host_dir);

                let max_dist = conn_half_thick.max(host_half_thick) * MAX_MITER_RATIO;

                // Default vertices (simple perpendicular offset at the junction endpoint).
                let junc_end = if is_end { conn_wall.end } else { conn_wall.start };
                let default_left = junc_end + conn_normal * conn_half_thick;
                let default_right = junc_end - conn_normal * conn_half_thick;

                let left = match left_int {
                    Some(pt) if pt.distance(junc_pt) < max_dist => pt,
                    _ => default_left,
                };
                let right = match right_int {
                    Some(pt) if pt.distance(junc_pt) < max_dist => pt,
                    _ => default_right,
                };

                joints.insert((conn_wall.id, is_end), JointVertices { left, right });
            }
        }
    }

    (joints, hubs)
}

// --- Miter geometry helpers (merged from wall_joints_miter.rs) ---

/// Two-wall miter joint with hub polygon to cover internal outline artifacts.
fn compute_two_wall_miter(
    waj_list: &[WallAtJunction],
    joints: &mut HashMap<(Uuid, bool), JointVertices>,
    hubs: &mut Vec<HubPolygon>,
    fill: egui::Color32,
) {
    let a = &waj_list[0];
    let b = &waj_list[1];

    // Junction point (average of the two from-points, which should be ~same).
    let junction = DVec2::new(
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
        Some(pt) if pt.distance(junction) < max_dist => pt,
        _ => a.left,
    };
    let a_right = match miter_rl {
        Some(pt) if pt.distance(junction) < max_dist => pt,
        _ => a.right,
    };

    // For wall B:
    let b_left = match miter_rl {
        Some(pt) if pt.distance(junction) < max_dist => pt,
        _ => b.left,
    };
    let b_right = match miter_lr {
        Some(pt) if pt.distance(junction) < max_dist => pt,
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
    let mut hub_vertices: Vec<DVec2> = candidates
        .iter()
        .filter_map(|opt| {
            opt.filter(|pt| pt.distance(junction) < max_dist)
        })
        .collect();

    if hub_vertices.len() >= 3 {
        // Sort by angle from centroid to ensure correct polygon winding.
        let cx = hub_vertices.iter().map(|p| p.x).sum::<f64>() / hub_vertices.len() as f64;
        let cy = hub_vertices.iter().map(|p| p.y).sum::<f64>() / hub_vertices.len() as f64;
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
fn compute_hub_polygon(
    waj_list: &[WallAtJunction],
    joints: &mut HashMap<(Uuid, bool), JointVertices>,
    hubs: &mut Vec<HubPolygon>,
    fill: egui::Color32,
) {
    let n = waj_list.len();
    let mut hub_vertices: Vec<DVec2> = Vec::new();

    // Walk walls in angular order (sorted by increasing atan2 angle).
    // Between consecutive walls i and i+1, the gap is bounded by
    // wall_i's CW-facing edge and wall_{i+1}'s CCW-facing edge.
    for i in 0..n {
        let next = (i + 1) % n;
        let wa = &waj_list[i];
        let wb = &waj_list[next];

        let miter = line_line_intersection(wa.cw_edge(), wa.dir, wb.ccw_edge(), wb.dir);

        let max_half = wa.half_thick.max(wb.half_thick);
        let junction_approx = (wa.cw_edge() + wb.ccw_edge()) / 2.0;
        let max_dist = max_half * MAX_MITER_RATIO;

        let pt = match miter {
            Some(p) if p.distance(junction_approx) < max_dist => p,
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
fn line_line_intersection(
    p1: DVec2,
    d1: DVec2,
    p2: DVec2,
    d2: DVec2,
) -> Option<DVec2> {
    let denom = d1.x * d2.y - d1.y * d2.x;
    if denom.abs() < 1e-10 {
        return None;
    }
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let t = (dx * d2.y - dy * d2.x) / denom;
    Some(DVec2::new(p1.x + t * d1.x, p1.y + t * d1.y))
}
