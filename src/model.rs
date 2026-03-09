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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LinePattern {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl LinePattern {
    pub const ALL: &[LinePattern] = &[LinePattern::Solid, LinePattern::Dashed, LinePattern::Dotted];

    pub fn label(&self) -> &'static str {
        match self {
            LinePattern::Solid => "Сплошная",
            LinePattern::Dashed => "Штриховая",
            LinePattern::Dotted => "Пунктирная",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ArrowMode {
    #[default]
    None,
    Forward,
    Backward,
    Both,
}

impl ArrowMode {
    pub const ALL: &[ArrowMode] = &[
        ArrowMode::None,
        ArrowMode::Forward,
        ArrowMode::Backward,
        ArrowMode::Both,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ArrowMode::None => "Нет",
            ArrowMode::Forward => "A-->>B",
            ArrowMode::Backward => "A<<--B",
            ArrowMode::Both => "↔ Обе",
        }
    }

    pub fn forward(&self) -> bool {
        matches!(self, ArrowMode::Forward | ArrowMode::Both)
    }

    pub fn backward(&self) -> bool {
        matches!(self, ArrowMode::Backward | ArrowMode::Both)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Uuid,
    pub point_a: Uuid,
    pub point_b: Uuid,
    /// Distance in mm. None = computed from point coordinates.
    pub distance_override: Option<f64>,
    /// Angle override in degrees. None = computed from coordinates.
    pub angle_override: Option<f64>,
    /// If true, label is displayed on the opposite side of the edge.
    #[serde(default)]
    pub label_flip_side: bool,
    /// If true, label text is rotated 180°.
    #[serde(default)]
    pub label_flip_text: bool,
    /// If true, measurement label is hidden on canvas.
    #[serde(default)]
    pub label_hidden: bool,
    /// Visual line pattern.
    #[serde(default)]
    pub line_pattern: LinePattern,
    /// Arrow direction.
    #[serde(default)]
    pub arrow_mode: ArrowMode,
}

impl Edge {
    pub fn new(point_a: Uuid, point_b: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            point_a,
            point_b,
            distance_override: None,
            angle_override: None,
            label_flip_side: false,
            label_flip_text: false,
            label_hidden: false,
            line_pattern: LinePattern::default(),
            arrow_mode: ArrowMode::default(),
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
    pub fn new(points: Vec<Uuid>, color: [u8; 4]) -> Self {
        Self {
            id: Uuid::new_v4(),
            points,
            color,
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
        /// Reveal width (mm)
        #[serde(default)]
        reveal_width: f64,
        /// Which polygon edge the swing arc is drawn on (0-based index)
        #[serde(default)]
        swing_edge: usize,
        /// If true, arc swings outward from polygon; if false, into polygon interior
        #[serde(default = "default_swing_outward")]
        swing_outward: bool,
        /// If true, hinge is at the other end of the swing edge (mirror)
        #[serde(default)]
        swing_mirrored: bool,
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

fn default_swing_outward() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    /// Polygon vertices (point IDs) defining the opening footprint.
    pub points: Vec<Uuid>,
    pub kind: OpeningKind,
    /// Fill color (RGBA).
    #[serde(default = "Opening::default_color")]
    pub color: [u8; 4],
}

impl Opening {
    pub fn new(points: Vec<Uuid>, kind: OpeningKind, color: [u8; 4]) -> Self {
        Self {
            id: Uuid::new_v4(),
            points,
            kind,
            color,
        }
    }

    fn default_color() -> [u8; 4] {
        [180, 160, 130, 200]
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
    /// Fill color (RGBA).
    #[serde(default = "Room::default_color")]
    pub color: [u8; 4],
    /// Offset of the room name label from the polygon centroid, in mm.
    /// `None` means default position (centroid).
    #[serde(default)]
    pub name_offset: Option<DVec2>,
}

impl Room {
    pub fn new(name: String, points: Vec<Uuid>, color: [u8; 4]) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            points,
            cutouts: Vec::new(),
            color,
            name_offset: None,
        }
    }

    pub fn default_color() -> [u8; 4] {
        [70, 130, 180, 45]
    }

    /// Area-weighted centroid of the room polygon (outer contour minus cutouts).
    pub fn centroid(&self, project: &Project) -> DVec2 {
        let outer = project.resolve_positions(&self.points);
        if outer.len() < 3 {
            if outer.is_empty() {
                return DVec2::ZERO;
            }
            let sum: DVec2 = outer.iter().copied().sum();
            return sum / outer.len() as f64;
        }

        let (outer_area, outer_c) = polygon_area_centroid(&outer);
        if outer_area < 1e-10 {
            let sum: DVec2 = outer.iter().copied().sum();
            return sum / outer.len() as f64;
        }

        let mut total_area = outer_area;
        let mut weighted = outer_c * outer_area;

        for cutout in &self.cutouts {
            let pts = project.resolve_positions(cutout);
            if pts.len() < 3 {
                continue;
            }
            let (cut_area, cut_c) = polygon_area_centroid(&pts);
            total_area -= cut_area;
            weighted -= cut_c * cut_area;
        }

        if total_area.abs() < 1e-10 {
            let sum: DVec2 = outer.iter().copied().sum();
            return sum / outer.len() as f64;
        }

        weighted / total_area
    }

    /// World position of the room name label (centroid + offset).
    pub fn name_position(&self, project: &Project) -> DVec2 {
        self.centroid(project) + self.name_offset.unwrap_or(DVec2::ZERO)
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

/// Compute absolute area and area-weighted centroid of a simple polygon.
fn polygon_area_centroid(polygon: &[DVec2]) -> (f64, DVec2) {
    let n = polygon.len();
    if n < 3 {
        return (0.0, DVec2::ZERO);
    }
    let mut signed_area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let cross = polygon[i].x * polygon[j].y - polygon[j].x * polygon[i].y;
        signed_area += cross;
        cx += (polygon[i].x + polygon[j].x) * cross;
        cy += (polygon[i].y + polygon[j].y) * cross;
    }
    signed_area *= 0.5;
    if signed_area.abs() < 1e-10 {
        return (0.0, DVec2::ZERO);
    }
    cx /= 6.0 * signed_area;
    cy /= 6.0 * signed_area;
    (signed_area.abs(), DVec2::new(cx, cy))
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

/// Check whether two points appear as consecutive vertices in a closed contour.
fn contour_has_edge(contour: &[Uuid], a: Uuid, b: Uuid) -> bool {
    let n = contour.len();
    for i in 0..n {
        let j = (i + 1) % n;
        if (contour[i] == a && contour[j] == b) || (contour[i] == b && contour[j] == a) {
            return true;
        }
    }
    false
}

/// Insert `new_id` between consecutive pair (a, b) in a single closed contour.
fn insert_in_closed_contour(contour: &mut Vec<Uuid>, a: Uuid, b: Uuid, new_id: Uuid) {
    let n = contour.len();
    let mut insertions = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        if (contour[i] == a && contour[j] == b) || (contour[i] == b && contour[j] == a) {
            // Insert new_id after index i. For wrap-around (j==0), insert at end.
            if j == 0 {
                insertions.push(n);
            } else {
                insertions.push(j);
            }
        }
    }
    // Insert in reverse order to keep indices valid.
    insertions.sort_unstable();
    insertions.reverse();
    for pos in insertions {
        contour.insert(pos, new_id);
    }
}

/// Insert a new point between (a, b) in all contours across rooms, walls, openings.
fn insert_point_in_contours(
    rooms: &mut [Room],
    walls: &mut [Wall],
    openings: &mut [Opening],
    a: Uuid,
    b: Uuid,
    new_id: Uuid,
) {
    for room in rooms.iter_mut() {
        insert_in_closed_contour(&mut room.points, a, b, new_id);
        for cutout in &mut room.cutouts {
            insert_in_closed_contour(cutout, a, b, new_id);
        }
    }
    for wall in walls.iter_mut() {
        insert_in_closed_contour(&mut wall.points, a, b, new_id);
    }
    for opening in openings.iter_mut() {
        insert_in_closed_contour(&mut opening.points, a, b, new_id);
    }
}

/// For a closed contour containing `id`, find the (prev, next) neighbor pairs
/// and append them to `pairs`.
fn collect_neighbor_pairs(contour: &[Uuid], id: Uuid, pairs: &mut Vec<(Uuid, Uuid)>) {
    let n = contour.len();
    for i in 0..n {
        if contour[i] == id {
            let prev = contour[if i == 0 { n - 1 } else { i - 1 }];
            let next = contour[(i + 1) % n];
            if prev != id && next != id && prev != next {
                pairs.push((prev, next));
            }
        }
    }
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
    #[serde(default = "default_door_reveal_width")]
    pub door_reveal_width: f64,
    pub window_height: f64,
    pub window_width: f64,
    pub window_sill_height: f64,
    pub window_reveal_width: f64,
    #[serde(default = "default_wall_color")]
    pub wall_color: [u8; 4],
    #[serde(default = "default_door_color")]
    pub door_color: [u8; 4],
    #[serde(default = "default_window_color")]
    pub window_color: [u8; 4],
}

fn default_door_reveal_width() -> f64 {
    0.0
}

fn default_wall_color() -> [u8; 4] {
    [180, 180, 180, 255]
}

fn default_door_color() -> [u8; 4] {
    [210, 170, 120, 200]
}

fn default_window_color() -> [u8; 4] {
    [120, 190, 230, 200]
}

impl Default for ProjectDefaults {
    fn default() -> Self {
        Self {
            point_height: 2700.0,
            door_height: 2100.0,
            door_width: 900.0,
            door_reveal_width: 0.0,
            window_height: 1400.0,
            window_width: 1200.0,
            window_sill_height: 900.0,
            window_reveal_width: 250.0,
            wall_color: default_wall_color(),
            door_color: default_door_color(),
            window_color: default_window_color(),
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

macro_rules! entity_lookup {
    ($field:ident, $T:ty, $get:ident, $get_mut:ident) => {
        pub fn $get(&self, id: Uuid) -> Option<&$T> {
            self.$field.iter().find(|x| x.id == id)
        }
        pub fn $get_mut(&mut self, id: Uuid) -> Option<&mut $T> {
            self.$field.iter_mut().find(|x| x.id == id)
        }
    };
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

    entity_lookup!(points, Point, point, point_mut);
    entity_lookup!(edges, Edge, edge, edge_mut);
    entity_lookup!(rooms, Room, room, room_mut);
    entity_lookup!(walls, Wall, wall, wall_mut);
    entity_lookup!(openings, Opening, opening, opening_mut);
    entity_lookup!(labels, Label, label, label_mut);

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

    /// Total area (mm²) of openings whose polygon touches this edge.
    pub fn openings_area_on_edge(&self, point_a: Uuid, point_b: Uuid) -> f64 {
        let mut total = 0.0;
        for opening in &self.openings {
            if contour_has_edge(&opening.points, point_a, point_b) {
                let (h, w) = match &opening.kind {
                    OpeningKind::Door { height, width, .. } => (*height, *width),
                    OpeningKind::Window { height, width, .. } => (*height, *width),
                };
                total += h * w;
            }
        }
        total
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
    /// that reference it. Kept for programmatic use; the UI uses `smart_remove_point`.
    #[cfg(test)]
    pub fn remove_point(&mut self, id: Uuid) {
        self.edges.retain(|e| e.point_a != id && e.point_b != id);
        self.rooms
            .retain(|r| !r.points.contains(&id) && !r.cutouts.iter().any(|c| c.contains(&id)));
        self.walls.retain(|w| !w.points.contains(&id));
        self.openings.retain(|o| !o.points.contains(&id));
        self.points.retain(|p| p.id != id);
    }

    /// Remove an edge by ID and cascade-delete all rooms, walls, and openings
    /// that have both edge endpoints as consecutive vertices in their contour.
    pub fn remove_edge(&mut self, id: Uuid) {
        if let Some(edge) = self.edge(id) {
            let a = edge.point_a;
            let b = edge.point_b;
            self.rooms.retain(|r| {
                !contour_has_edge(&r.points, a, b)
                    && !r.cutouts.iter().any(|c| contour_has_edge(c, a, b))
            });
            self.walls.retain(|w| !contour_has_edge(&w.points, a, b));
            self.openings.retain(|o| !contour_has_edge(&o.points, a, b));
        }
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

    // --- Split edge ---

    /// Split an edge by inserting a new point at `position`.
    ///
    /// - Removes the original edge A→B.
    /// - Creates a new point with height interpolated from A and B.
    /// - Creates two new edges: A→New and New→B (no overrides).
    /// - Updates all contours (rooms, cutouts, walls, openings) that have
    ///   the consecutive pair (A,B) in either direction, inserting New between them.
    ///
    /// Returns the ID of the new point.
    pub fn split_edge(&mut self, edge_id: Uuid, position: DVec2) -> Uuid {
        let edge = self.edges.iter().find(|e| e.id == edge_id).unwrap();
        let pa_id = edge.point_a;
        let pb_id = edge.point_b;

        // Interpolate height based on projection parameter t.
        let pa = self.point(pa_id).unwrap();
        let pb = self.point(pb_id).unwrap();
        let (t, _) = project_onto_segment(position, pa.position, pb.position);
        let height = pa.height * (1.0 - t) + pb.height * t;

        // Create new point.
        let new_point = Point::new(position, height);
        let new_id = new_point.id;
        self.points.push(new_point);

        // Remove original edge.
        self.edges.retain(|e| e.id != edge_id);

        // Create two new edges (no overrides).
        self.ensure_edge(pa_id, new_id);
        self.ensure_edge(new_id, pb_id);

        // Update all contours.
        insert_point_in_contours(
            &mut self.rooms,
            &mut self.walls,
            &mut self.openings,
            pa_id,
            pb_id,
            new_id,
        );

        new_id
    }

    // --- Smart point removal ---

    /// Remove a point "smartly": excise it from contours rather than
    /// cascade-deleting them.
    ///
    /// - For each contour containing the point, find its neighbors (prev, next)
    ///   and record them as new edge pairs to create.
    /// - Remove the point UUID from all contour lists.
    /// - Remove degenerate objects (fewer than 3 points).
    /// - Remove all edges connected to this point.
    /// - Create new edges for the recorded neighbor pairs.
    /// - Remove the point itself.
    pub fn smart_remove_point(&mut self, id: Uuid) {
        let mut new_edge_pairs: Vec<(Uuid, Uuid)> = Vec::new();

        // Process rooms (main contours + cutouts).
        for room in &mut self.rooms {
            collect_neighbor_pairs(&room.points, id, &mut new_edge_pairs);
            room.points.retain(|pid| *pid != id);
            for cutout in &mut room.cutouts {
                collect_neighbor_pairs(cutout, id, &mut new_edge_pairs);
                cutout.retain(|pid| *pid != id);
            }
            room.cutouts.retain(|c| c.len() >= 3);
        }
        self.rooms.retain(|r| r.points.len() >= 3);

        // Process walls.
        for wall in &mut self.walls {
            collect_neighbor_pairs(&wall.points, id, &mut new_edge_pairs);
            wall.points.retain(|pid| *pid != id);
        }
        self.walls.retain(|w| w.points.len() >= 3);

        // Process openings.
        for opening in &mut self.openings {
            collect_neighbor_pairs(&opening.points, id, &mut new_edge_pairs);
            opening.points.retain(|pid| *pid != id);
        }
        self.openings.retain(|o| o.points.len() >= 3);

        // Remove all edges connected to this point.
        self.edges.retain(|e| e.point_a != id && e.point_b != id);

        // Create new edges for neighbor pairs.
        for (a, b) in &new_edge_pairs {
            self.ensure_edge(*a, *b);
        }

        // Remove the point itself.
        self.points.retain(|p| p.id != id);
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

        let room = Room::new("Test Room".to_string(), ids, Room::default_color());
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

        project.rooms.push(Room::new("Room".to_string(), ids.clone(), Room::default_color()));
        project.walls.push(Wall::new(ids.clone(), [180, 180, 180, 255]));
        project.openings.push(Opening::new(
            ids.clone(),
            OpeningKind::Door {
                height: 2100.0,
                width: 900.0,
                reveal_width: 0.0,
                swing_edge: 0,
                swing_outward: true,
                swing_mirrored: false,
            },
            [210, 170, 120, 200],
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
        let room = Room::new("R".to_string(), ids.clone(), Room::default_color());
        let room_id = room.id;
        project.rooms.push(room);

        project.remove_room(room_id);
        assert_eq!(project.rooms.len(), 0);
        assert_eq!(project.points.len(), 3);
    }

    // -- Split edge tests --

    #[test]
    fn test_split_edge_basic() {
        let mut project = Project::new("test".to_string());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        project.points.push(make_point(a, 0.0, 0.0));
        project.points.push(make_point(b, 1000.0, 0.0));
        let edge_id = project.ensure_edge(a, b);

        let new_id = project.split_edge(edge_id, DVec2::new(500.0, 0.0));

        // Original edge gone, two new edges created.
        assert!(project.find_edge(a, b).is_none());
        assert!(project.find_edge(a, new_id).is_some());
        assert!(project.find_edge(new_id, b).is_some());
        assert_eq!(project.edges.len(), 2);

        // New point at correct position.
        let np = project.point(new_id).unwrap();
        assert!((np.position.x - 500.0).abs() < 0.01);
        assert!((np.position.y).abs() < 0.01);
    }

    #[test]
    fn test_split_edge_updates_room_contour() {
        let (mut project, room_id) = make_rect_project();
        // Room has 4 points: [ids[0], ids[1], ids[2], ids[3]]
        let room = project.room(room_id).unwrap();
        let pa = room.points[0];
        let pb = room.points[1];

        let edge_id = project.find_edge(pa, pb).unwrap().id;
        let new_id = project.split_edge(edge_id, DVec2::new(1000.0, 0.0));

        let room = project.room(room_id).unwrap();
        assert_eq!(room.points.len(), 5, "room should have 5 points after split");
        // New point should be between pa and pb.
        let idx_pa = room.points.iter().position(|&id| id == pa).unwrap();
        let idx_new = (idx_pa + 1) % room.points.len();
        assert_eq!(room.points[idx_new], new_id);
    }

    #[test]
    fn test_split_edge_wrap_around() {
        // Room [A, B, C], split edge C→A (wrap-around).
        let mut project = Project::new("test".to_string());
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let positions = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1000.0, 0.0),
            DVec2::new(500.0, 1000.0),
        ];
        for (i, &id) in ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: positions[i],
                height: 2700.0,
            });
        }
        project.ensure_contour_edges(&ids);
        project.rooms.push(Room::new("R".to_string(), ids.clone(), Room::default_color()));

        let edge_id = project.find_edge(ids[2], ids[0]).unwrap().id;
        let mid = (positions[2] + positions[0]) / 2.0;
        let new_id = project.split_edge(edge_id, mid);

        let room = &project.rooms[0];
        assert_eq!(room.points.len(), 4);
        // New point should be after C (last original) = at end of list.
        assert_eq!(room.points[3], new_id);
    }

    #[test]
    fn test_split_edge_height_interpolation() {
        let mut project = Project::new("test".to_string());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        project.points.push(Point {
            id: a,
            position: DVec2::new(0.0, 0.0),
            height: 2000.0,
        });
        project.points.push(Point {
            id: b,
            position: DVec2::new(1000.0, 0.0),
            height: 3000.0,
        });
        let edge_id = project.ensure_edge(a, b);

        let new_id = project.split_edge(edge_id, DVec2::new(500.0, 0.0));
        let np = project.point(new_id).unwrap();
        assert!(
            (np.height - 2500.0).abs() < 0.01,
            "expected 2500, got {}",
            np.height
        );
    }

    // -- Smart remove point tests --

    #[test]
    fn test_smart_remove_point_from_quad() {
        let (mut project, room_id) = make_rect_project();
        let room = project.room(room_id).unwrap();
        let to_delete = room.points[1]; // second point
        let prev = room.points[0];
        let next = room.points[2];

        project.smart_remove_point(to_delete);

        // Room survives with 3 points.
        assert_eq!(project.rooms.len(), 1);
        let room = &project.rooms[0];
        assert_eq!(room.points.len(), 3);
        assert!(!room.points.contains(&to_delete));

        // New edge between prev and next should exist.
        assert!(project.find_edge(prev, next).is_some());

        // Point itself removed.
        assert!(project.point(to_delete).is_none());
    }

    #[test]
    fn test_smart_remove_point_triangle_degenerate() {
        let mut project = Project::new("test".to_string());
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        for &id in &ids {
            project
                .points
                .push(Point::new(DVec2::new(0.0, 0.0), 2700.0));
            project.points.last_mut().unwrap().id = id;
        }
        project.ensure_contour_edges(&ids);
        project.rooms.push(Room::new("R".to_string(), ids.clone(), Room::default_color()));

        project.smart_remove_point(ids[1]);

        // Room becomes degenerate (2 points) and should be removed.
        assert_eq!(project.rooms.len(), 0);
        assert_eq!(project.points.len(), 2);
    }

    #[test]
    fn test_smart_remove_point_cutout_degenerate() {
        let (mut project, room_id) = make_rect_project();

        // Add a triangular cutout.
        let cutout_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        for &id in &cutout_ids {
            project
                .points
                .push(Point::new(DVec2::new(500.0, 500.0), 2700.0));
            project.points.last_mut().unwrap().id = id;
        }
        project.ensure_contour_edges(&cutout_ids);
        project
            .rooms
            .iter_mut()
            .find(|r| r.id == room_id)
            .unwrap()
            .cutouts
            .push(cutout_ids.clone());

        project.smart_remove_point(cutout_ids[0]);

        // Room survives, cutout is removed (degenerate).
        assert_eq!(project.rooms.len(), 1);
        let room = &project.rooms[0];
        assert_eq!(room.cutouts.len(), 0);
        assert_eq!(room.points.len(), 4); // main contour untouched
    }

    #[test]
    fn test_smart_remove_point_edges_only() {
        // Point with only edges, no contours.
        let mut project = Project::new("test".to_string());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        project.points.push(make_point(a, 0.0, 0.0));
        project.points.push(make_point(b, 1000.0, 0.0));
        project.points.push(make_point(c, 2000.0, 0.0));
        project.ensure_edge(a, b);
        project.ensure_edge(b, c);

        project.smart_remove_point(b);

        assert_eq!(project.points.len(), 2);
        assert_eq!(project.edges.len(), 0); // both edges removed, no contour→no new edge
        assert!(project.point(b).is_none());
    }
}
