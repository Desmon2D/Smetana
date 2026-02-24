use glam::DVec2;

/// Shoelace formula — absolute area of a simple polygon in mm².
pub fn shoelace_area(polygon: &[DVec2]) -> f64 {
    let mut area = 0.0;
    let n = polygon.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].x * polygon[j].y - polygon[j].x * polygon[i].y;
    }
    (area / 2.0).abs()
}

/// Distance from point `p` to the line segment from `a` to `b`.
pub fn distance_to_segment(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let (_, proj) = project_onto_segment(p, a, b);
    p.distance(proj)
}

/// Project point `p` onto the line segment from `a` to `b`.
/// Returns (t, projected_point) where t is in [0, 1].
pub fn project_onto_segment(p: DVec2, a: DVec2, b: DVec2) -> (f64, DVec2) {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-12 {
        return (0.0, a);
    }
    let t = (p - a).dot(ab) / len_sq;
    let t = t.clamp(0.0, 1.0);
    (t, a + ab * t)
}

/// Ray-casting point-in-polygon test.
/// Returns `true` if `point` is inside the polygon defined by `polygon` vertices.
pub fn point_in_polygon(point: DVec2, polygon: &[DVec2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Compute the interior angle at the junction of two edges.
/// `prev` is the incoming edge, `curr` is the outgoing edge.
/// Returns the angle in degrees.
pub fn compute_angle_from_coords(
    prev: &super::Edge,
    curr: &super::Edge,
    points: &[super::Point],
) -> f64 {
    use uuid::Uuid;

    // Find the shared point between prev and curr
    let shared_id = if prev.point_b == curr.point_a || prev.point_b == curr.point_b {
        prev.point_b
    } else if prev.point_a == curr.point_a || prev.point_a == curr.point_b {
        prev.point_a
    } else {
        return 0.0;
    };

    let prev_other = if prev.point_a == shared_id {
        prev.point_b
    } else {
        prev.point_a
    };

    let curr_other = if curr.point_a == shared_id {
        curr.point_b
    } else {
        curr.point_a
    };

    let find = |id: Uuid| points.iter().find(|p| p.id == id);

    let (Some(shared), Some(a), Some(b)) = (find(shared_id), find(prev_other), find(curr_other))
    else {
        return 0.0;
    };

    let v1 = a.position - shared.position;
    let v2 = b.position - shared.position;

    let angle = v1.y.atan2(v1.x) - v2.y.atan2(v2.x);
    angle.to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shoelace_area_square() {
        let polygon = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1000.0, 0.0),
            DVec2::new(1000.0, 1000.0),
            DVec2::new(0.0, 1000.0),
        ];
        let area = shoelace_area(&polygon);
        assert!(
            (area - 1_000_000.0).abs() < 0.01,
            "expected 1000000, got {area}"
        );
    }

    #[test]
    fn test_shoelace_area_triangle() {
        let polygon = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(2000.0, 0.0),
            DVec2::new(0.0, 1000.0),
        ];
        let area = shoelace_area(&polygon);
        assert!(
            (area - 1_000_000.0).abs() < 0.01,
            "expected 1000000, got {area}"
        );
    }

    #[test]
    fn test_distance_to_segment_perpendicular() {
        let p = DVec2::new(500.0, 300.0);
        let a = DVec2::new(0.0, 0.0);
        let b = DVec2::new(1000.0, 0.0);
        let dist = distance_to_segment(p, a, b);
        assert!((dist - 300.0).abs() < 0.01, "expected 300, got {dist}");
    }

    #[test]
    fn test_point_in_polygon_inside() {
        let polygon = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1000.0, 0.0),
            DVec2::new(1000.0, 1000.0),
            DVec2::new(0.0, 1000.0),
        ];
        assert!(point_in_polygon(DVec2::new(500.0, 500.0), &polygon));
    }

    #[test]
    fn test_point_in_polygon_outside() {
        let polygon = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1000.0, 0.0),
            DVec2::new(1000.0, 1000.0),
            DVec2::new(0.0, 1000.0),
        ];
        assert!(!point_in_polygon(DVec2::new(1500.0, 500.0), &polygon));
    }
}
