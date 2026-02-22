use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A T-junction on one side of a wall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideJunction {
    /// ID of the connecting wall.
    pub wall_id: Uuid,
    /// Parametric position along the wall (0.0 = start, 1.0 = end).
    pub t: f64,
}

/// Properties of a single section of a wall side (between junctions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionData {
    pub length: f64,
    pub height_start: f64,
    pub height_end: f64,
}

impl SectionData {
    /// Gross area in mm² (trapezoid formula).
    pub fn gross_area(&self) -> f64 {
        self.length * (self.height_start + self.height_end) / 2.0
    }
}

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
pub struct SideData {
    /// Side length in mm (user-editable)
    pub length: f64,
    /// Height at the start end of the wall (mm)
    pub height_start: f64,
    /// Height at the end end of the wall (mm)
    pub height_end: f64,
    /// T-junctions on this side, sorted by t.
    #[serde(default)]
    pub junctions: Vec<SideJunction>,
    /// Section properties. Empty = no junctions (use whole-side data).
    /// When junctions exist: junctions.len() + 1 entries.
    #[serde(default)]
    pub sections: Vec<SectionData>,
}

impl SideData {
    pub fn new(length: f64, height_start: f64, height_end: f64) -> Self {
        Self {
            length,
            height_start,
            height_end,
            junctions: Vec::new(),
            sections: Vec::new(),
        }
    }

    /// Gross area in mm² (trapezoid formula)
    pub fn gross_area(&self) -> f64 {
        self.length * (self.height_start + self.height_end) / 2.0
    }

    /// Returns true if this side has T-junctions (and thus sections).
    pub fn has_sections(&self) -> bool {
        !self.junctions.is_empty()
    }

    /// Number of sections (1 if no junctions, N+1 if N junctions).
    pub fn section_count(&self) -> usize {
        if self.junctions.is_empty() { 1 } else { self.junctions.len() + 1 }
    }

    /// Insert a junction and recompute sections.
    pub fn add_junction(&mut self, wall_id: Uuid, t: f64) {
        // Insert sorted by t
        let pos = self.junctions.iter().position(|j| j.t > t).unwrap_or(self.junctions.len());
        self.junctions.insert(pos, SideJunction { wall_id, t });
        self.recompute_sections();
    }

    /// Remove a junction by connecting wall ID and recompute sections.
    pub fn remove_junction(&mut self, wall_id: Uuid) {
        self.junctions.retain(|j| j.wall_id != wall_id);
        if self.junctions.is_empty() {
            self.sections.clear();
        } else {
            self.recompute_sections();
        }
    }

    /// Recompute section data from junctions.
    fn recompute_sections(&mut self) {
        let n = self.junctions.len();
        // Boundary t values: [0.0, t1, t2, ..., 1.0]
        let mut boundaries = Vec::with_capacity(n + 2);
        boundaries.push(0.0);
        for j in &self.junctions {
            boundaries.push(j.t);
        }
        boundaries.push(1.0);

        self.sections.clear();
        for i in 0..boundaries.len() - 1 {
            let t_start = boundaries[i];
            let t_end = boundaries[i + 1];
            let section_length = (t_end - t_start) * self.length;
            let h_start = self.height_start + (self.height_end - self.height_start) * t_start;
            let h_end = self.height_start + (self.height_end - self.height_start) * t_end;
            self.sections.push(SectionData {
                length: section_length,
                height_start: h_start,
                height_end: h_end,
            });
        }
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
    /// Left side looking from start to end
    pub left_side: SideData,
    /// Right side looking from start to end
    pub right_side: SideData,
    /// IDs of attached openings
    pub openings: Vec<Uuid>,
}

impl Wall {
    pub fn new(start: Point2D, end: Point2D) -> Self {
        let length = start.distance_to(end);
        Self {
            id: Uuid::new_v4(),
            start,
            end,
            thickness: 200.0,
            left_side: SideData::new(length, 2700.0, 2700.0),
            right_side: SideData::new(length, 2700.0, 2700.0),
            openings: Vec::new(),
        }
    }

    /// Wall centerline length in mm (for canvas rendering)
    pub fn length(&self) -> f64 {
        self.start.distance_to(self.end)
    }

    /// Gross area of the left side in mm²
    pub fn left_area(&self) -> f64 {
        self.left_side.gross_area()
    }

    /// Gross area of the right side in mm²
    pub fn right_area(&self) -> f64 {
        self.right_side.gross_area()
    }
}
