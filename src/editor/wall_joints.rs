use std::collections::HashMap;

use eframe::egui;
use uuid::Uuid;

use crate::editor::canvas::Canvas;
use crate::model::Wall;
use super::wall_joints_miter::{compute_two_wall_miter, compute_hub_polygon, line_line_intersection};

/// Merge epsilon for detecting shared endpoints (mm).
const MERGE_EPS: f64 = 5.0;

/// Maximum miter extension as a multiple of the thicker wall's half-thickness.
pub(super) const MAX_MITER_RATIO: f32 = 3.0;

/// Computed joint vertices for one end of a wall (screen coordinates).
pub struct JointVertices {
    /// Adjusted left-side vertex (left = positive normal direction).
    pub left: egui::Pos2,
    /// Adjusted right-side vertex (right = negative normal direction).
    pub right: egui::Pos2,
}

/// A hub polygon at a wall junction, drawn on top to cover internal outline artifacts.
pub struct HubPolygon {
    pub vertices: Vec<egui::Pos2>,
    pub fill: egui::Color32,
}

/// Per-wall direction info at a junction (precomputed).
pub(super) struct WallAtJunction {
    pub(super) wall_id: Uuid,
    pub(super) is_end: bool,
    /// Outgoing angle from junction (radians).
    pub(super) angle: f32,
    /// Half-thickness in screen pixels.
    pub(super) half_thick: f32,
    /// Left edge point at the junction (perpendicular offset, screen coords).
    pub(super) left: egui::Pos2,
    /// Right edge point at the junction (perpendicular offset, screen coords).
    pub(super) right: egui::Pos2,
    /// Outgoing unit direction from junction.
    pub(super) dir: egui::Vec2,
}

/// Compute adjusted joint vertices for all wall endpoints and hub polygons.
pub fn compute_joints(
    walls: &[Wall],
    canvas: &Canvas,
    center: egui::Pos2,
) -> (HashMap<(Uuid, bool), JointVertices>, Vec<HubPolygon>) {
    let mut joints: HashMap<(Uuid, bool), JointVertices> = HashMap::new();
    let mut hubs: Vec<HubPolygon> = Vec::new();

    if walls.is_empty() {
        return (joints, hubs);
    }

    // Group wall endpoints by junction (merge within MERGE_EPS).
    // Each junction: list of (wall_id, is_end, world_point).
    let mut junction_groups: Vec<(f64, f64, Vec<(Uuid, bool)>)> = Vec::new();

    for wall in walls {
        for &is_end in &[false, true] {
            let pt = if is_end { wall.end } else { wall.start };
            let mut found = false;
            for (jx, jy, members) in &mut junction_groups {
                let dx = pt.x - *jx;
                let dy = pt.y - *jy;
                if (dx * dx + dy * dy).sqrt() < MERGE_EPS {
                    members.push((wall.id, is_end));
                    found = true;
                    break;
                }
            }
            if !found {
                junction_groups.push((pt.x, pt.y, vec![(wall.id, is_end)]));
            }
        }
    }

    let wall_fill = egui::Color32::from_rgb(140, 140, 145);

    for (_jx, _jy, members) in &junction_groups {
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

            let from_screen = canvas.world_to_screen(
                egui::pos2(from.x as f32, from.y as f32),
                center,
            );
            let to_screen = canvas.world_to_screen(
                egui::pos2(to.x as f32, to.y as f32),
                center,
            );

            // Note: "outgoing" means from junction toward the wall's other end.
            // For is_end=false: junction is at wall.start, outgoing toward wall.end.
            // For is_end=true: junction is at wall.end, outgoing toward wall.start.
            // But for the wall quad, we need the direction start→end always,
            // and the junction point is `from_screen`.
            // The outgoing direction from the junction:
            let out_dx = to_screen.x - from_screen.x;
            let out_dy = to_screen.y - from_screen.y;
            let out_len = (out_dx * out_dx + out_dy * out_dy).sqrt();
            if out_len < 0.1 {
                continue;
            }
            let dir = egui::vec2(out_dx / out_len, out_dy / out_len);
            let angle = out_dy.atan2(out_dx);

            let half_thick = (wall.thickness as f32 * canvas.zoom) / 2.0;

            // Left normal: rotate dir 90° CCW → (-dir.y, dir.x)
            // But we need to match the convention in draw_walls:
            // nx = -dy/len * half_thick, ny = dx/len * half_thick
            // where (dx, dy) = end_screen - start_screen (wall direction start→end).
            //
            // For is_end=false (junction at start): wall dir = (out_dx, out_dy)
            //   left normal = (-out_dy, out_dx) / out_len  (matches draw_walls: nx=-dy/len, ny=dx/len)
            // For is_end=true (junction at end): wall dir start→end = (-out_dx, -out_dy)
            //   left normal = (out_dy, -out_dx) / out_len  (= -(-out_dy, out_dx)/len)
            let normal_left = if !is_end {
                egui::vec2(-dir.y, dir.x)
            } else {
                egui::vec2(dir.y, -dir.x)
            };

            let left = egui::pos2(
                from_screen.x + normal_left.x * half_thick,
                from_screen.y + normal_left.y * half_thick,
            );
            let right = egui::pos2(
                from_screen.x - normal_left.x * half_thick,
                from_screen.y - normal_left.y * half_thick,
            );

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
        for (side_data, sign) in [(&host_wall.left_side, 1.0f32), (&host_wall.right_side, -1.0f32)] {
            if side_data.junctions.is_empty() {
                continue;
            }

            let host_start_s = canvas.world_to_screen(
                egui::pos2(host_wall.start.x as f32, host_wall.start.y as f32),
                center,
            );
            let host_end_s = canvas.world_to_screen(
                egui::pos2(host_wall.end.x as f32, host_wall.end.y as f32),
                center,
            );

            let host_dx = host_end_s.x - host_start_s.x;
            let host_dy = host_end_s.y - host_start_s.y;
            let host_len = (host_dx * host_dx + host_dy * host_dy).sqrt();
            if host_len < 0.1 {
                continue;
            }

            // Host wall direction and left normal (looking start→end).
            let host_dir = egui::vec2(host_dx / host_len, host_dy / host_len);
            let host_normal = egui::vec2(-host_dir.y, host_dir.x);
            let host_half_thick = (host_wall.thickness as f32 * canvas.zoom) / 2.0;

            // A point on the host wall's side edge line.
            let edge_point = egui::pos2(
                host_start_s.x + host_normal.x * host_half_thick * sign,
                host_start_s.y + host_normal.y * host_half_thick * sign,
            );

            for junction in &side_data.junctions {
                let conn_wall = match walls.iter().find(|w| w.id == junction.wall_id) {
                    Some(w) => w,
                    None => continue,
                };

                let conn_start_s = canvas.world_to_screen(
                    egui::pos2(conn_wall.start.x as f32, conn_wall.start.y as f32),
                    center,
                );
                let conn_end_s = canvas.world_to_screen(
                    egui::pos2(conn_wall.end.x as f32, conn_wall.end.y as f32),
                    center,
                );

                // Junction point on the host wall's side edge.
                let junc_s = egui::pos2(
                    host_start_s.x + host_dx * junction.t as f32
                        + host_normal.x * host_half_thick * sign,
                    host_start_s.y + host_dy * junction.t as f32
                        + host_normal.y * host_half_thick * sign,
                );

                // Determine which end of the connecting wall is at the junction.
                let is_end = (conn_end_s - junc_s).length() < (conn_start_s - junc_s).length();

                // Skip if already handled by endpoint junction grouping.
                if joints.contains_key(&(conn_wall.id, is_end)) {
                    continue;
                }

                // Connecting wall direction (start→end) and left normal.
                let conn_dx = conn_end_s.x - conn_start_s.x;
                let conn_dy = conn_end_s.y - conn_start_s.y;
                let conn_len = (conn_dx * conn_dx + conn_dy * conn_dy).sqrt();
                if conn_len < 0.1 {
                    continue;
                }

                let conn_dir = egui::vec2(conn_dx / conn_len, conn_dy / conn_len);
                let conn_normal = egui::vec2(-conn_dir.y, conn_dir.x);
                let conn_half_thick = (conn_wall.thickness as f32 * canvas.zoom) / 2.0;

                // Connecting wall's left/right edge lines (parallel to centerline).
                let left_edge_pt = egui::pos2(
                    conn_start_s.x + conn_normal.x * conn_half_thick,
                    conn_start_s.y + conn_normal.y * conn_half_thick,
                );
                let right_edge_pt = egui::pos2(
                    conn_start_s.x - conn_normal.x * conn_half_thick,
                    conn_start_s.y - conn_normal.y * conn_half_thick,
                );

                // Intersect each edge with the host wall's side edge line.
                let left_int = line_line_intersection(left_edge_pt, conn_dir, edge_point, host_dir);
                let right_int =
                    line_line_intersection(right_edge_pt, conn_dir, edge_point, host_dir);

                let max_dist = conn_half_thick.max(host_half_thick) * MAX_MITER_RATIO;

                // Default vertices (simple perpendicular offset at the junction endpoint).
                let junc_end_s = if is_end { conn_end_s } else { conn_start_s };
                let default_left = egui::pos2(
                    junc_end_s.x + conn_normal.x * conn_half_thick,
                    junc_end_s.y + conn_normal.y * conn_half_thick,
                );
                let default_right = egui::pos2(
                    junc_end_s.x - conn_normal.x * conn_half_thick,
                    junc_end_s.y - conn_normal.y * conn_half_thick,
                );

                let left = match left_int {
                    Some(pt) if (pt - junc_s).length() < max_dist => pt,
                    _ => default_left,
                };
                let right = match right_int {
                    Some(pt) if (pt - junc_s).length() < max_dist => pt,
                    _ => default_right,
                };

                joints.insert((conn_wall.id, is_end), JointVertices { left, right });
            }
        }
    }

    (joints, hubs)
}
