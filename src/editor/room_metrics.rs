use glam::DVec2;
use crate::model::{Room, Wall, WallSide, project_onto_segment};

/// Computed metrics for a room.
#[derive(Debug, Clone)]
pub struct RoomMetrics {
    /// Inner polygon vertices (wall centerlines offset inward by half-thickness)
    pub inner_polygon: Vec<DVec2>,
    /// Gross floor area in mm² (centerline polygon — includes wall volume)
    pub gross_area: f64,
    /// Net floor area in mm² (interior polygon — clear floor area)
    pub net_area: f64,
    /// Inner perimeter in mm (sum of room-facing side section lengths)
    pub perimeter: f64,
}

/// Compute the inner polygon, areas, and perimeter for a room.
///
/// - **Net area**: offset each wall centerline inward by half-thickness on
///   the room-facing side, intersect consecutive offset lines, then Shoelace.
/// - **Gross area**: centerline polygon (shared wall endpoints, no offset).
/// - **Perimeter**: sum of section lengths on the room-facing side (accounts
///   for junction wall thicknesses being excluded).
pub fn compute_room_metrics(room: &Room, walls: &[Wall]) -> Option<RoomMetrics> {
    if room.wall_ids.len() < 3 {
        return None;
    }

    let has_segments = room.wall_segments.len() == room.wall_ids.len();

    // Build offset lines for each wall (inner polygon)
    let mut offset_segments: Vec<(DVec2, DVec2)> = Vec::new();

    for (i, wall_id) in room.wall_ids.iter().enumerate() {
        let wall = walls.iter().find(|w| w.id == *wall_id)?;
        let side = room.wall_sides[i];
        let half_t = wall.thickness / 2.0;

        // Use segment endpoints when available (correct for T-junction
        // segments), otherwise fall back to full wall extent.
        let (raw_start, raw_end) = if has_segments {
            room.wall_segments[i]
        } else {
            (wall.start, wall.end)
        };

        // Wall direction vector (always from wall.start->wall.end for
        // consistent normal orientation regardless of segment extent)
        let d = wall.end - wall.start;
        let len = d.length();
        if len < 1e-6 {
            return None;
        }

        // Project segment endpoints onto the wall's centerline.
        let (seg_start, seg_end) = {
            let (_, ps) = project_onto_segment(raw_start, wall.start, wall.end);
            let (_, pe) = project_onto_segment(raw_end, wall.start, wall.end);
            (ps, pe)
        };

        // Unit normal pointing toward room interior
        let normal = match side {
            WallSide::Left => DVec2::new(-d.y / len, d.x / len),
            WallSide::Right => DVec2::new(d.y / len, -d.x / len),
        };

        let offset_start = seg_start + normal * half_t;
        let offset_end = seg_end + normal * half_t;

        offset_segments.push((offset_start, offset_end));
    }

    // Intersect consecutive offset lines to get inner polygon vertices
    let n = offset_segments.len();
    let mut inner_polygon = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + 1) % n;
        let (a1, a2) = offset_segments[i];
        let (b1, b2) = offset_segments[j];

        match line_intersection(a1, a2, b1, b2) {
            Some(pt) => inner_polygon.push(pt),
            None => inner_polygon.push(offset_segments[i].1),
        }
    }

    // Net area from inner polygon (Shoelace formula), minus column walls
    let polygon_area = shoelace_area(&inner_polygon);
    let column_area = column_wall_area(room, walls, &inner_polygon);
    let net_area = polygon_area - column_area;

    // Gross area from centerline polygon (segment endpoints, no offset)
    let gross_area = centerline_area(room, walls).unwrap_or(polygon_area);

    // Perimeter from room-facing side section lengths
    // (uses section lengths which exclude junction wall thicknesses)
    let mut perimeter = 0.0;
    for (i, wall_id) in room.wall_ids.iter().enumerate() {
        let wall = walls.iter().find(|w| w.id == *wall_id)?;
        let side_data = match room.wall_sides[i] {
            WallSide::Left => &wall.left_side,
            WallSide::Right => &wall.right_side,
        };

        if has_segments {
            // Only sum sections within the segment's t-range
            let (seg_start, seg_end) = room.wall_segments[i];
            let wall_len = wall.length();
            if wall_len < 1e-6 {
                continue;
            }
            let t_seg_start = project_t(seg_start, wall);
            let t_seg_end = project_t(seg_end, wall);
            let t_lo = t_seg_start.min(t_seg_end);
            let t_hi = t_seg_start.max(t_seg_end);

            // Build boundary t values from junctions
            let mut boundaries = vec![0.0_f64];
            for j in &side_data.junctions {
                boundaries.push(j.t);
            }
            boundaries.push(1.0);

            for (k, section) in side_data.sections.iter().enumerate() {
                if k >= boundaries.len() - 1 {
                    break;
                }
                let s_lo = boundaries[k];
                let s_hi = boundaries[k + 1];
                // Include section if it overlaps the segment range
                if s_hi > t_lo + 0.001 && s_lo < t_hi - 0.001 {
                    perimeter += section.length;
                }
            }
        } else {
            let section_sum: f64 = side_data.sections.iter().map(|s| s.length).sum();
            perimeter += section_sum;
        }
    }

    Some(RoomMetrics {
        inner_polygon,
        gross_area,
        net_area,
        perimeter,
    })
}

/// Project a point onto a wall's centerline and return the parametric t value.
fn project_t(point: DVec2, wall: &Wall) -> f64 {
    let d = wall.end - wall.start;
    let len_sq = d.length_squared();
    if len_sq < 1e-12 {
        return 0.0;
    }
    (point - wall.start).dot(d) / len_sq
}

/// Shoelace formula — absolute area of a simple polygon.
fn shoelace_area(polygon: &[DVec2]) -> f64 {
    let mut area = 0.0;
    let n = polygon.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].x * polygon[j].y - polygon[j].x * polygon[i].y;
    }
    (area / 2.0).abs()
}

/// Compute the area of the centerline polygon (shared wall endpoints, no
/// inward offset). This represents the gross area including wall volume.
fn centerline_area(room: &Room, walls: &[Wall]) -> Option<f64> {
    let n = room.wall_ids.len();
    if n < 3 {
        return None;
    }

    let has_segments = room.wall_segments.len() == n;

    let mut vertices = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + 1) % n;

        if has_segments {
            // Use the junction between consecutive segments: segment i's
            // end should be near segment j's start (they share a vertex
            // in the room cycle).  Use midpoint for robustness.
            let seg_i_end = room.wall_segments[i].1;
            let seg_j_start = room.wall_segments[j].0;
            vertices.push((seg_i_end + seg_j_start) / 2.0);
        } else {
            // Legacy fallback: find closest endpoint pair between walls
            let wall_i = walls.iter().find(|w| w.id == room.wall_ids[i])?;
            let wall_j = walls.iter().find(|w| w.id == room.wall_ids[j])?;

            let candidates = [
                (wall_i.start, wall_j.start),
                (wall_i.start, wall_j.end),
                (wall_i.end, wall_j.start),
                (wall_i.end, wall_j.end),
            ];

            let &(best_pt, _) = candidates
                .iter()
                .min_by(|(a1, b1), (a2, b2)| {
                    a1.distance(*b1)
                        .partial_cmp(&a2.distance(*b2))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;

            vertices.push(best_pt);
        }
    }

    Some(shoelace_area(&vertices))
}

/// Total cross-section area of column/partition walls inside the room.
fn column_wall_area(room: &Room, walls: &[Wall], inner_polygon: &[DVec2]) -> f64 {
    use std::collections::HashSet;

    if inner_polygon.len() < 3 {
        return 0.0;
    }

    let room_wall_set: HashSet<uuid::Uuid> = room.wall_ids.iter().copied().collect();

    let mut total = 0.0;
    for wall in walls {
        if room_wall_set.contains(&wall.id) {
            continue;
        }

        let mid = (wall.start + wall.end) / 2.0;

        if point_in_polygon(mid, inner_polygon) {
            total += wall.length() * wall.thickness;
        }
    }

    total
}

/// Ray-casting point-in-polygon test.
fn point_in_polygon(point: DVec2, polygon: &[DVec2]) -> bool {
    let n = polygon.len();
    let mut inside = false;
    let mut j = n - 1;

    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];

        if (pi.y > point.y) != (pj.y > point.y) {
            let x_intersect = (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x;
            if point.x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }

    inside
}

/// Intersect two infinite lines defined by points (a1,a2) and (b1,b2).
/// Returns None if lines are parallel.
fn line_intersection(a1: DVec2, a2: DVec2, b1: DVec2, b2: DVec2) -> Option<DVec2> {
    let d1 = a2 - a1;
    let d2 = b2 - b1;
    let denom = d1.perp_dot(d2);
    if denom.abs() < 1e-10 {
        return None;
    }
    let t = (b1 - a1).perp_dot(d2) / denom;
    Some(a1 + d1 * t)
}
