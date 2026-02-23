use crate::model::{Point2D, Room, Wall, WallSide};

/// Computed metrics for a room.
#[derive(Debug, Clone)]
pub struct RoomMetrics {
    /// Inner polygon vertices (wall centerlines offset inward by half-thickness)
    pub inner_polygon: Vec<Point2D>,
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

    // Build offset lines for each wall (inner polygon)
    let mut offset_segments: Vec<(Point2D, Point2D)> = Vec::new();

    for (i, wall_id) in room.wall_ids.iter().enumerate() {
        let wall = walls.iter().find(|w| w.id == *wall_id)?;
        let side = room.wall_sides[i];
        let half_t = wall.thickness / 2.0;

        // Wall direction vector
        let dx = wall.end.x - wall.start.x;
        let dy = wall.end.y - wall.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return None;
        }

        // Unit normal pointing toward room interior
        let (nx, ny) = match side {
            WallSide::Left => (-dy / len, dx / len),
            WallSide::Right => (dy / len, -dx / len),
        };

        let offset_start = Point2D::new(wall.start.x + nx * half_t, wall.start.y + ny * half_t);
        let offset_end = Point2D::new(wall.end.x + nx * half_t, wall.end.y + ny * half_t);

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

    // Gross area from centerline polygon (wall endpoints, no offset)
    let gross_area = centerline_area(room, walls).unwrap_or(polygon_area);

    // Perimeter from room-facing side section lengths
    // (uses section lengths which exclude junction wall thicknesses)
    let mut perimeter = 0.0;
    for (i, wall_id) in room.wall_ids.iter().enumerate() {
        let wall = walls.iter().find(|w| w.id == *wall_id)?;
        let side = match room.wall_sides[i] {
            WallSide::Left => &wall.left_side,
            WallSide::Right => &wall.right_side,
        };
        let section_sum: f64 = side.sections.iter().map(|s| s.length).sum();
        perimeter += section_sum;
    }

    Some(RoomMetrics {
        inner_polygon,
        gross_area,
        net_area,
        perimeter,
    })
}

/// Shoelace formula — absolute area of a simple polygon.
fn shoelace_area(polygon: &[Point2D]) -> f64 {
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

    let mut vertices = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + 1) % n;
        let wall_i = walls.iter().find(|w| w.id == room.wall_ids[i])?;
        let wall_j = walls.iter().find(|w| w.id == room.wall_ids[j])?;

        // Find the closest pair of endpoints between consecutive walls
        let candidates = [
            (wall_i.start, wall_j.start),
            (wall_i.start, wall_j.end),
            (wall_i.end, wall_j.start),
            (wall_i.end, wall_j.end),
        ];

        let &(best_pt, _) = candidates
            .iter()
            .min_by(|(a1, b1), (a2, b2)| {
                a1.distance_to(*b1)
                    .partial_cmp(&a2.distance_to(*b2))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;

        vertices.push(best_pt);
    }

    Some(shoelace_area(&vertices))
}

/// Total cross-section area of column/partition walls inside the room.
///
/// A column wall is a wall that is NOT part of the room's boundary contour
/// but whose midpoint lies inside the room's inner polygon. Both sides of
/// such a wall face the room interior, so its cross-section (length × thickness)
/// is subtracted from the net floor area.
fn column_wall_area(room: &Room, walls: &[Wall], inner_polygon: &[Point2D]) -> f64 {
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

        let mid = Point2D::new(
            (wall.start.x + wall.end.x) / 2.0,
            (wall.start.y + wall.end.y) / 2.0,
        );

        if point_in_polygon(mid, inner_polygon) {
            total += wall.length() * wall.thickness;
        }
    }

    total
}

/// Ray-casting point-in-polygon test.
fn point_in_polygon(point: Point2D, polygon: &[Point2D]) -> bool {
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
fn line_intersection(a1: Point2D, a2: Point2D, b1: Point2D, b2: Point2D) -> Option<Point2D> {
    let d1x = a2.x - a1.x;
    let d1y = a2.y - a1.y;
    let d2x = b2.x - b1.x;
    let d2y = b2.y - b1.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-10 {
        return None;
    }

    let t = ((b1.x - a1.x) * d2y - (b1.y - a1.y) * d2x) / denom;
    Some(Point2D::new(a1.x + t * d1x, a1.y + t * d1y))
}
