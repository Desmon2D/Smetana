use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D {
    /// X coordinate in world space (mm)
    pub x: f64,
    /// Y coordinate in world space (mm)
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Point2D) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    /// Distance from this point to the line segment from `a` to `b`.
    pub fn distance_to_segment(self, a: Point2D, b: Point2D) -> f64 {
        let (_t, proj) = self.project_onto_segment(a, b);
        self.distance_to(proj)
    }

    /// Project this point onto the line segment from `a` to `b`.
    /// Returns (t, projected_point) where t is in [0, 1].
    pub fn project_onto_segment(self, a: Point2D, b: Point2D) -> (f64, Point2D) {
        let ab_x = b.x - a.x;
        let ab_y = b.y - a.y;
        let len_sq = ab_x * ab_x + ab_y * ab_y;
        if len_sq < 1e-12 {
            return (0.0, a);
        }
        let t = ((self.x - a.x) * ab_x + (self.y - a.y) * ab_y) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let proj = Point2D::new(a.x + t * ab_x, a.y + t * ab_y);
        (t, proj)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    /// Start point in world coordinates (mm)
    pub start: Point2D,
    /// End point in world coordinates (mm)
    pub end: Point2D,
    /// Wall thickness (mm)
    pub thickness: f64,
    /// Height at the start edge (mm)
    pub height_start: f64,
    /// Height at the end edge (mm)
    pub height_end: f64,
    /// IDs of attached openings
    pub openings: Vec<Uuid>,
}

impl Wall {
    pub fn new(start: Point2D, end: Point2D) -> Self {
        Self {
            id: Uuid::new_v4(),
            start,
            end,
            thickness: 200.0,
            height_start: 2700.0,
            height_end: 2700.0,
            openings: Vec::new(),
        }
    }

    /// Wall length in mm
    pub fn length(&self) -> f64 {
        self.start.distance_to(self.end)
    }

    /// Gross wall area in mm² (trapezoid formula for different heights)
    pub fn gross_area(&self) -> f64 {
        self.length() * (self.height_start + self.height_end) / 2.0
    }
}
