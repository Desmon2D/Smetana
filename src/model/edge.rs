use glam::DVec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Point;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Uuid,
    pub point_a: Uuid,
    pub point_b: Uuid,
    /// Distance in mm. None = computed from point coordinates.
    pub distance_override: Option<f64>,
    /// Angle override in degrees. None = computed from coordinates.
    pub angle_override: Option<f64>,
}

impl Edge {
    pub fn new(point_a: Uuid, point_b: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            point_a,
            point_b,
            distance_override: None,
            angle_override: None,
        }
    }

    /// Effective distance: override if set, otherwise Euclidean from coordinates.
    pub fn distance(&self, points: &[Point]) -> f64 {
        if let Some(d) = self.distance_override {
            return d;
        }
        let a = points.iter().find(|p| p.id == self.point_a);
        let b = points.iter().find(|p| p.id == self.point_b);
        match (a, b) {
            (Some(a), Some(b)) => a.position.distance(b.position),
            _ => 0.0,
        }
    }

    /// Effective angle between this edge and the previous edge.
    /// `prev_edge` defines the incoming direction.
    pub fn angle(&self, prev_edge: &Edge, points: &[Point]) -> f64 {
        if let Some(a) = self.angle_override {
            return a;
        }
        compute_angle_from_coords(prev_edge, self, points)
    }
}

/// Compute the interior angle at the junction of two edges.
/// `prev` is the incoming edge, `curr` is the outgoing edge.
/// Returns the angle in degrees.
pub fn compute_angle_from_coords(prev: &Edge, curr: &Edge, points: &[Point]) -> f64 {
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

// --- Geometry utilities (preserved from old model) ---

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_point(id: Uuid, x: f64, y: f64) -> Point {
        Point {
            id,
            position: DVec2::new(x, y),
            height: 2700.0,
        }
    }

    #[test]
    fn test_edge_distance_computed() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let points = vec![make_point(id_a, 0.0, 0.0), make_point(id_b, 3000.0, 4000.0)];
        let edge = Edge::new(id_a, id_b);
        let dist = edge.distance(&points);
        assert!((dist - 5000.0).abs() < 0.01, "expected 5000, got {dist}");
    }

    #[test]
    fn test_edge_distance_override() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let points = vec![make_point(id_a, 0.0, 0.0), make_point(id_b, 3000.0, 4000.0)];
        let mut edge = Edge::new(id_a, id_b);
        edge.distance_override = Some(9999.0);
        let dist = edge.distance(&points);
        assert!((dist - 9999.0).abs() < 0.01, "expected 9999, got {dist}");
    }

    #[test]
    fn test_shoelace_area_square() {
        // 1000×1000mm square = 1,000,000 mm²
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
        // Right triangle: base 2000, height 1000 → area 1,000,000 mm²
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
