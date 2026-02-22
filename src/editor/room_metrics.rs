use crate::model::{Point2D, Room, Wall, WallSide};

/// Computed metrics for a room.
#[derive(Debug, Clone)]
pub struct RoomMetrics {
    /// Inner polygon vertices (wall centerlines offset inward by half-thickness)
    pub inner_polygon: Vec<Point2D>,
    /// Floor area in mm²
    pub area: f64,
    /// Inner perimeter in mm
    pub perimeter: f64,
}

/// Compute the inner polygon, area, and perimeter for a room.
///
/// For each wall in the room's contour, offset the wall centerline inward
/// by half-thickness on the room-facing side. Then intersect consecutive
/// offset lines to get the inner polygon vertices.
pub fn compute_room_metrics(room: &Room, walls: &[Wall]) -> Option<RoomMetrics> {
    if room.wall_ids.len() < 3 {
        return None;
    }

    // Build offset lines for each wall
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

        // Unit normal: perpendicular to wall direction
        // Left normal (when looking start→end): (-dy, dx) / len
        // Right normal: (dy, -dx) / len
        let (nx, ny) = match side {
            WallSide::Left => (-dy / len, dx / len),
            WallSide::Right => (dy / len, -dx / len),
        };

        // Offset the wall centerline inward by half-thickness
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
            // If lines are parallel, use the endpoint of the first segment
            None => inner_polygon.push(offset_segments[i].1),
        }
    }

    // Compute area using Shoelace formula
    let mut area = 0.0;
    for i in 0..inner_polygon.len() {
        let j = (i + 1) % inner_polygon.len();
        let p1 = inner_polygon[i];
        let p2 = inner_polygon[j];
        area += p1.x * p2.y - p2.x * p1.y;
    }
    let area = (area / 2.0).abs();

    // Compute perimeter from room-facing side lengths
    let mut perimeter = 0.0;
    for (i, wall_id) in room.wall_ids.iter().enumerate() {
        let wall = walls.iter().find(|w| w.id == *wall_id)?;
        let side = match room.wall_sides[i] {
            WallSide::Left => &wall.left_side,
            WallSide::Right => &wall.right_side,
        };
        perimeter += side.length;
    }

    Some(RoomMetrics {
        inner_polygon,
        area,
        perimeter,
    })
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
