use glam::DVec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Point
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub id: Uuid,
    /// Canvas position in mm (world coordinates)
    pub position: DVec2,
    /// Ceiling height at this point in mm
    pub height: f64,
}

impl Point {
    pub fn new(position: DVec2, height: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            height,
        }
    }
}

// ---------------------------------------------------------------------------
// Edge
// ---------------------------------------------------------------------------

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
    pub fn angle(&self, prev_edge: &Edge, points: &[Point]) -> f64 {
        if let Some(a) = self.angle_override {
            return a;
        }
        compute_angle_from_coords(prev_edge, self, points)
    }
}

// ---------------------------------------------------------------------------
// Wall
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    /// Polygon vertices (point IDs) defining the wall shape.
    pub points: Vec<Uuid>,
    /// Fill color (RGBA).
    pub color: [u8; 4],
}

impl Wall {
    pub fn new(points: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            points,
            color: [180, 180, 180, 255], // default gray
        }
    }
}

// ---------------------------------------------------------------------------
// Opening
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpeningKind {
    Door {
        /// Door height (mm)
        height: f64,
        /// Door width (mm)
        width: f64,
    },
    Window {
        /// Window height (mm)
        height: f64,
        /// Window width (mm)
        width: f64,
        /// Height from floor to window sill (mm)
        sill_height: f64,
        /// Reveal width (mm)
        reveal_width: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    /// Polygon vertices (point IDs) defining the opening footprint.
    pub points: Vec<Uuid>,
    pub kind: OpeningKind,
}

impl Opening {
    pub fn new(points: Vec<Uuid>, kind: OpeningKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            points,
            kind,
        }
    }
}

// ---------------------------------------------------------------------------
// Label
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: Uuid,
    pub text: String,
    pub position: DVec2,
    /// Display font size in points (default 14.0)
    pub font_size: f64,
    /// Rotation in radians (default 0.0)
    pub rotation: f64,
}

impl Label {
    pub fn new(text: String, position: DVec2) -> Self {
        Self {
            id: Uuid::new_v4(),
            text,
            position,
            font_size: 14.0,
            rotation: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Room
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    /// Outer contour: ordered point IDs forming a closed polygon.
    pub points: Vec<Uuid>,
    /// Cutouts (columns, shafts): each is an ordered list of point IDs.
    #[serde(default)]
    pub cutouts: Vec<Vec<Uuid>>,
}

impl Room {
    pub fn new(name: String, points: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            points,
            cutouts: Vec::new(),
        }
    }

    /// Floor area in mm² (outer contour minus cutouts).
    pub fn floor_area(&self, project: &Project) -> f64 {
        let outer = Self::contour_area(&self.points, project);
        let cutout_area: f64 = self
            .cutouts
            .iter()
            .map(|c| Self::contour_area(c, project))
            .sum();
        (outer - cutout_area).max(0.0)
    }

    /// Perimeter in mm (sum of outer contour edge distances).
    pub fn perimeter(&self, project: &Project) -> f64 {
        let n = self.points.len();
        (0..n)
            .map(|i| {
                let j = (i + 1) % n;
                project
                    .find_edge(self.points[i], self.points[j])
                    .map(|e| e.distance(&project.points))
                    .unwrap_or_else(|| {
                        let a = project.point(self.points[i]);
                        let b = project.point(self.points[j]);
                        match (a, b) {
                            (Some(a), Some(b)) => a.position.distance(b.position),
                            _ => 0.0,
                        }
                    })
            })
            .sum()
    }

    fn contour_area(contour: &[Uuid], project: &Project) -> f64 {
        let has_overrides = contour.windows(2).any(|w| {
            project
                .find_edge(w[0], w[1])
                .is_some_and(|e| e.distance_override.is_some() || e.angle_override.is_some())
        }) || (contour.len() >= 2
            && project
                .find_edge(*contour.last().unwrap(), contour[0])
                .is_some_and(|e| e.distance_override.is_some() || e.angle_override.is_some()));

        if has_overrides {
            Self::area_from_measurements(contour, project)
        } else {
            shoelace_area(&project.resolve_positions(contour))
        }
    }

    fn area_from_measurements(contour: &[Uuid], project: &Project) -> f64 {
        let n = contour.len();
        if n < 3 {
            return 0.0;
        }

        let mut distances = Vec::with_capacity(n);
        let mut angles = Vec::with_capacity(n);

        for i in 0..n {
            let j = (i + 1) % n;
            let edge = match project.find_edge(contour[i], contour[j]) {
                Some(e) => e,
                None => return shoelace_area(&project.resolve_positions(contour)),
            };
            distances.push(edge.distance(&project.points));

            let k = (j + 1) % n;
            let next_edge = match project.find_edge(contour[j], contour[k]) {
                Some(e) => e,
                None => return shoelace_area(&project.resolve_positions(contour)),
            };
            angles.push(edge.angle(next_edge, &project.points));
        }

        let mut vertices = Vec::with_capacity(n);
        vertices.push(DVec2::ZERO);

        let mut cumulative_angle: f64 = 0.0;

        for i in 0..n - 1 {
            cumulative_angle += std::f64::consts::PI - angles[i].to_radians();
            let dir = DVec2::new(cumulative_angle.cos(), cumulative_angle.sin());
            let prev = *vertices.last().unwrap();
            vertices.push(prev + dir * distances[i]);
        }

        shoelace_area(&vertices)
    }
}

// ---------------------------------------------------------------------------
// Geometry free functions
// ---------------------------------------------------------------------------

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
pub fn compute_angle_from_coords(prev: &Edge, curr: &Edge, points: &[Point]) -> f64 {
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

// ---------------------------------------------------------------------------
// ProjectDefaults
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDefaults {
    pub point_height: f64,
    pub door_height: f64,
    pub door_width: f64,
    pub window_height: f64,
    pub window_width: f64,
    pub window_sill_height: f64,
    pub window_reveal_width: f64,
}

impl Default for ProjectDefaults {
    fn default() -> Self {
        Self {
            point_height: 2700.0,
            door_height: 2100.0,
            door_width: 900.0,
            window_height: 1400.0,
            window_width: 1200.0,
            window_sill_height: 900.0,
            window_reveal_width: 250.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub rooms: Vec<Room>,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub defaults: ProjectDefaults,
}

impl Project {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            points: Vec::new(),
            edges: Vec::new(),
            rooms: Vec::new(),
            walls: Vec::new(),
            openings: Vec::new(),
            labels: Vec::new(),
            defaults: ProjectDefaults::default(),
        }
    }

    // --- Lookup by ID ---

    pub fn point(&self, id: Uuid) -> Option<&Point> {
        self.points.iter().find(|p| p.id == id)
    }

    pub fn point_mut(&mut self, id: Uuid) -> Option<&mut Point> {
        self.points.iter_mut().find(|p| p.id == id)
    }

    pub fn edge(&self, id: Uuid) -> Option<&Edge> {
        self.edges.iter().find(|e| e.id == id)
    }

    pub fn edge_mut(&mut self, id: Uuid) -> Option<&mut Edge> {
        self.edges.iter_mut().find(|e| e.id == id)
    }

    pub fn room(&self, id: Uuid) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }

    pub fn wall(&self, id: Uuid) -> Option<&Wall> {
        self.walls.iter().find(|w| w.id == id)
    }

    pub fn opening(&self, id: Uuid) -> Option<&Opening> {
        self.openings.iter().find(|o| o.id == id)
    }

    pub fn opening_mut(&mut self, id: Uuid) -> Option<&mut Opening> {
        self.openings.iter_mut().find(|o| o.id == id)
    }

    /// Resolve a list of point IDs to their world-space positions.
    pub fn resolve_positions(&self, ids: &[Uuid]) -> Vec<DVec2> {
        ids.iter()
            .filter_map(|id| self.point(*id).map(|p| p.position))
            .collect()
    }

    // --- Edge lookup (direction-agnostic) ---

    pub fn find_edge(&self, a: Uuid, b: Uuid) -> Option<&Edge> {
        self.edges
            .iter()
            .find(|e| (e.point_a == a && e.point_b == b) || (e.point_a == b && e.point_b == a))
    }

    #[allow(dead_code)]
    pub fn find_edge_mut(&mut self, a: Uuid, b: Uuid) -> Option<&mut Edge> {
        self.edges
            .iter_mut()
            .find(|e| (e.point_a == a && e.point_b == b) || (e.point_a == b && e.point_b == a))
    }

    /// Ensure an edge exists between two points. Returns the edge ID.
    pub fn ensure_edge(&mut self, point_a: Uuid, point_b: Uuid) -> Uuid {
        if let Some(edge) = self.find_edge(point_a, point_b) {
            return edge.id;
        }
        let edge = Edge::new(point_a, point_b);
        let id = edge.id;
        self.edges.push(edge);
        id
    }

    /// Ensure all edges exist for a closed contour of points.
    pub fn ensure_contour_edges(&mut self, points: &[Uuid]) {
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            self.ensure_edge(points[i], points[j]);
        }
    }

    // --- Mutation (cascade delete) ---

    /// Remove a point and cascade-delete all edges, rooms, walls, and openings
    /// that reference it.
    pub fn remove_point(&mut self, id: Uuid) {
        self.edges.retain(|e| e.point_a != id && e.point_b != id);
        self.rooms
            .retain(|r| !r.points.contains(&id) && !r.cutouts.iter().any(|c| c.contains(&id)));
        self.walls.retain(|w| !w.points.contains(&id));
        self.openings.retain(|o| !o.points.contains(&id));
        self.points.retain(|p| p.id != id);
    }

    /// Remove an edge by ID.
    pub fn remove_edge(&mut self, id: Uuid) {
        self.edges.retain(|e| e.id != id);
    }

    /// Remove a room by ID.
    pub fn remove_room(&mut self, id: Uuid) {
        self.rooms.retain(|r| r.id != id);
    }

    /// Remove a wall by ID.
    pub fn remove_wall(&mut self, id: Uuid) {
        self.walls.retain(|w| w.id != id);
    }

    /// Remove an opening by ID.
    pub fn remove_opening(&mut self, id: Uuid) {
        self.openings.retain(|o| o.id != id);
    }

    /// Remove a label by ID.
    pub fn remove_label(&mut self, id: Uuid) {
        self.labels.retain(|l| l.id != id);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    // -- Edge tests --

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

    // -- Geometry tests --

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

    // -- Room tests --

    fn make_rect_project() -> (Project, Uuid) {
        let mut project = Project::new("test".to_string());
        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let positions = [
            DVec2::new(0.0, 0.0),
            DVec2::new(2000.0, 0.0),
            DVec2::new(2000.0, 3000.0),
            DVec2::new(0.0, 3000.0),
        ];

        for (i, &id) in ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: positions[i],
                height: 2700.0,
            });
        }

        project.ensure_contour_edges(&ids);

        let room = Room::new("Test Room".to_string(), ids);
        let room_id = room.id;
        project.rooms.push(room);

        (project, room_id)
    }

    #[test]
    fn test_room_perimeter() {
        let (project, room_id) = make_rect_project();
        let room = project.room(room_id).unwrap();
        let perimeter = room.perimeter(&project);
        assert!(
            (perimeter - 10000.0).abs() < 0.01,
            "expected 10000, got {perimeter}"
        );
    }

    #[test]
    fn test_room_floor_area() {
        let (project, room_id) = make_rect_project();
        let room = project.room(room_id).unwrap();
        let area = room.floor_area(&project);
        assert!(
            (area - 6_000_000.0).abs() < 0.01,
            "expected 6000000, got {area}"
        );
    }

    #[test]
    fn test_room_floor_area_with_cutout() {
        let (mut project, room_id) = make_rect_project();

        let cutout_ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let cutout_positions = [
            DVec2::new(500.0, 500.0),
            DVec2::new(1000.0, 500.0),
            DVec2::new(1000.0, 1000.0),
            DVec2::new(500.0, 1000.0),
        ];

        for (i, &id) in cutout_ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: cutout_positions[i],
                height: 2700.0,
            });
        }

        project.ensure_contour_edges(&cutout_ids);

        let room = project.rooms.iter_mut().find(|r| r.id == room_id).unwrap();
        room.cutouts.push(cutout_ids);

        let room = project.room(room_id).unwrap();
        let area = room.floor_area(&project);
        assert!(
            (area - 5_750_000.0).abs() < 0.01,
            "expected 5750000, got {area}"
        );
    }

    // -- Project tests --

    #[test]
    fn test_ensure_edge_dedup() {
        let mut project = Project::new("test".to_string());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        project
            .points
            .push(Point::new(DVec2::new(0.0, 0.0), 2700.0));
        project
            .points
            .push(Point::new(DVec2::new(1000.0, 0.0), 2700.0));
        project.points[0].id = a;
        project.points[1].id = b;

        let id1 = project.ensure_edge(a, b);
        let id2 = project.ensure_edge(a, b);
        let id3 = project.ensure_edge(b, a); // reversed direction
        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
        assert_eq!(project.edges.len(), 1);
    }

    #[test]
    fn test_remove_point_cascades() {
        let mut project = Project::new("test".to_string());

        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let positions = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1000.0, 0.0),
            DVec2::new(1000.0, 1000.0),
            DVec2::new(0.0, 1000.0),
        ];
        for (i, &id) in ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: positions[i],
                height: 2700.0,
            });
        }

        project.ensure_contour_edges(&ids);
        assert_eq!(project.edges.len(), 4);

        project.rooms.push(Room::new("Room".to_string(), ids.clone()));
        project.walls.push(Wall::new(ids.clone()));
        project.openings.push(Opening::new(
            ids.clone(),
            OpeningKind::Door {
                height: 2100.0,
                width: 900.0,
            },
        ));

        assert_eq!(project.rooms.len(), 1);
        assert_eq!(project.walls.len(), 1);
        assert_eq!(project.openings.len(), 1);

        project.remove_point(ids[0]);

        assert_eq!(project.points.len(), 3);
        assert_eq!(project.edges.len(), 2);
        assert_eq!(project.rooms.len(), 0);
        assert_eq!(project.walls.len(), 0);
        assert_eq!(project.openings.len(), 0);
    }

    #[test]
    fn test_remove_room_only() {
        let mut project = Project::new("test".to_string());
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        for &id in &ids {
            project.points.push(Point::new(DVec2::ZERO, 2700.0));
            project.points.last_mut().unwrap().id = id;
        }
        let room = Room::new("R".to_string(), ids.clone());
        let room_id = room.id;
        project.rooms.push(room);

        project.remove_room(room_id);
        assert_eq!(project.rooms.len(), 0);
        assert_eq!(project.points.len(), 3);
    }
}
