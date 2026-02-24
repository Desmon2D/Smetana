use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Point;
use super::geometry::compute_angle_from_coords;

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

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec2;

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
}
