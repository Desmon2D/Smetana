# Smetana Redesign: Technical Specification

## 1. Data Structures

### 1.1 Point

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub id: Uuid,
    /// Canvas position in mm (used for rendering and default distance/angle computation)
    pub position: DVec2,
    /// Height at this point in mm (editable by user)
    pub height: f64,
}
```

### 1.2 Edge

Edges are stored in a dedicated collection. Each edge connects two points.

```rust
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
    /// Effective distance: override if set, otherwise Euclidean from coordinates.
    pub fn distance(&self, points: &[Point]) -> f64 {
        if let Some(d) = self.distance_override {
            return d;
        }
        let a = points.iter().find(|p| p.id == self.point_a).unwrap();
        let b = points.iter().find(|p| p.id == self.point_b).unwrap();
        a.position.distance(b.position)
    }

    /// Effective angle between this edge and the previous edge.
    /// `prev_edge` defines the incoming direction.
    pub fn angle(&self, prev_edge: &Edge, points: &[Point]) -> f64 {
        if let Some(a) = self.angle_override {
            return a;
        }
        // Compute from coordinates (see Section 3.1)
        compute_angle_from_coords(prev_edge, self, points)
    }
}
```

### 1.3 Room

```rust
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
```

### 1.4 Wall (Visual)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    /// Polygon vertices defining the wall shape on the canvas.
    pub points: Vec<Uuid>,
    /// Fill color (RGBA).
    pub color: [u8; 4],
}
```

### 1.5 Opening

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    /// Polygon vertices defining the opening footprint.
    pub points: Vec<Uuid>,
    pub kind: OpeningKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpeningKind {
    Door {
        height: f64,   // mm
        width: f64,    // mm
    },
    Window {
        height: f64,       // mm
        width: f64,        // mm
        sill_height: f64,  // mm
        reveal_width: f64, // mm
    },
}
```

### 1.6 Project (Top-Level)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub rooms: Vec<Room>,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub defaults: ProjectDefaults,

    // Services (unchanged concept, but keyed differently)
    /// Services assigned to room edges: HashMap<(RoomId, EdgeId), Vec<AssignedService>>
    pub edge_services: HashMap<(Uuid, Uuid), Vec<AssignedService>>,
    /// Services assigned to openings: HashMap<OpeningId, Vec<AssignedService>>
    pub opening_services: HashMap<Uuid, Vec<AssignedService>>,
    /// Services assigned to rooms (floor): HashMap<RoomId, Vec<AssignedService>>
    pub room_services: HashMap<Uuid, Vec<AssignedService>>,
}
```

## 2. Edge Management

### 2.1 Implicit Edge Creation

Edges are created automatically when objects reference point pairs:

```rust
impl Project {
    /// Ensure an edge exists between two points. Returns the edge ID.
    /// If an edge already exists (in either direction), returns its ID.
    pub fn ensure_edge(&mut self, point_a: Uuid, point_b: Uuid) -> Uuid {
        // Check for existing edge (a->b or b->a)
        if let Some(edge) = self.edges.iter().find(|e|
            (e.point_a == point_a && e.point_b == point_b) ||
            (e.point_a == point_b && e.point_b == point_a)
        ) {
            return edge.id;
        }
        // Create new edge
        let edge = Edge {
            id: Uuid::new_v4(),
            point_a,
            point_b,
            distance_override: None,
            angle_override: None,
        };
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
}
```

### 2.2 Edge Lookup

```rust
impl Project {
    /// Find edge between two points (direction-agnostic).
    pub fn find_edge(&self, a: Uuid, b: Uuid) -> Option<&Edge> {
        self.edges.iter().find(|e|
            (e.point_a == a && e.point_b == b) ||
            (e.point_a == b && e.point_b == a)
        )
    }

    /// Find edge between two points (mutable).
    pub fn find_edge_mut(&mut self, a: Uuid, b: Uuid) -> Option<&mut Edge> {
        self.edges.iter_mut().find(|e|
            (e.point_a == a && e.point_b == b) ||
            (e.point_a == b && e.point_b == a)
        )
    }
}
```

## 3. Calculation Algorithms

### 3.1 Angle Computation from Coordinates

```rust
/// Compute the interior angle at the junction of two edges.
/// `prev` is the incoming edge, `curr` is the outgoing edge.
/// `shared` is the shared point ID.
fn compute_angle_from_coords(prev: &Edge, curr: &Edge, points: &[Point]) -> f64 {
    let shared_id = /* find the shared point between prev and curr */;
    let prev_other = /* the other endpoint of prev */;
    let curr_other = /* the other endpoint of curr */;

    let shared_pos = find_point(points, shared_id).position;
    let a_pos = find_point(points, prev_other).position;
    let b_pos = find_point(points, curr_other).position;

    let v1 = a_pos - shared_pos;  // incoming direction (reversed)
    let v2 = b_pos - shared_pos;  // outgoing direction

    let angle = v1.y.atan2(v1.x) - v2.y.atan2(v2.x);
    angle.to_degrees().rem_euclid(360.0)
}
```

### 3.2 Room Floor Area

Two computation modes depending on whether overrides exist:

```rust
impl Room {
    pub fn floor_area(&self, project: &Project) -> f64 {
        let outer = self.contour_area(&self.points, project);
        let cutout_area: f64 = self.cutouts.iter()
            .map(|c| self.contour_area(c, project))
            .sum();
        (outer - cutout_area).max(0.0)
    }

    fn contour_area(&self, contour: &[Uuid], project: &Project) -> f64 {
        let has_overrides = /* check if any edge in contour has distance or angle override */;

        if has_overrides {
            // Build polygon from distances + angles (measurement-based)
            self.area_from_measurements(contour, project)
        } else {
            // Use coordinate-based Shoelace (faster, no accumulation error)
            self.area_from_coordinates(contour, project)
        }
    }
}
```

#### 3.2.1 Area from Coordinates (Default)

Standard Shoelace formula on point positions:

```rust
fn area_from_coordinates(&self, contour: &[Uuid], project: &Project) -> f64 {
    let positions: Vec<DVec2> = contour.iter()
        .map(|id| project.point(*id).position)
        .collect();
    shoelace_area(&positions)
}

fn shoelace_area(polygon: &[DVec2]) -> f64 {
    let mut area = 0.0;
    let n = polygon.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].x * polygon[j].y - polygon[j].x * polygon[i].y;
    }
    (area / 2.0).abs()
}
```

#### 3.2.2 Area from Measurements (When Overrides Exist)

Build a polygon from edge distances and angles, then Shoelace:

```rust
fn area_from_measurements(&self, contour: &[Uuid], project: &Project) -> f64 {
    let n = contour.len();
    if n < 3 { return 0.0; }

    // Collect edge distances and angles for the contour
    let mut distances = Vec::with_capacity(n);
    let mut angles = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + 1) % n;
        let edge = project.find_edge(contour[i], contour[j]).unwrap();
        distances.push(edge.distance(&project.points));

        // Angle at vertex j (between edge i->j and edge j->k)
        let k = (j + 1) % n;
        let next_edge = project.find_edge(contour[j], contour[k]).unwrap();
        angles.push(edge.angle(next_edge, &project.points));
    }

    // Reconstruct polygon vertices
    let mut vertices = Vec::with_capacity(n);
    vertices.push(DVec2::ZERO);

    let mut cumulative_angle: f64 = 0.0;

    for i in 0..n - 1 {
        cumulative_angle += std::f64::consts::PI - angles[i].to_radians();
        let dir = DVec2::new(cumulative_angle.cos(), cumulative_angle.sin());
        let prev = vertices.last().unwrap();
        vertices.push(*prev + dir * distances[i]);
    }

    shoelace_area(&vertices)
}
```

### 3.3 Room Perimeter

```rust
impl Room {
    pub fn perimeter(&self, project: &Project) -> f64 {
        let n = self.points.len();
        (0..n).map(|i| {
            let j = (i + 1) % n;
            let edge = project.find_edge(self.points[i], self.points[j]).unwrap();
            edge.distance(&project.points)
        }).sum()
    }
}
```

### 3.4 Wall Area per Room Edge

For services assigned to a room edge (wall painting, plastering, etc.):

```rust
/// Net wall area for a specific edge of a room.
/// Subtracts any openings that share points with this edge.
pub fn edge_wall_area(
    room: &Room,
    edge_point_a: Uuid,
    edge_point_b: Uuid,
    project: &Project,
) -> f64 {
    let edge = project.find_edge(edge_point_a, edge_point_b).unwrap();
    let distance = edge.distance(&project.points);

    let height_a = project.point(edge_point_a).height;
    let height_b = project.point(edge_point_b).height;
    let avg_height = (height_a + height_b) / 2.0;

    let gross_area = distance * avg_height;

    // Subtract openings on this edge
    let opening_area: f64 = project.openings.iter()
        .filter(|o| opening_on_edge(o, edge_point_a, edge_point_b))
        .map(|o| o.kind.height() * o.kind.width())
        .sum();

    (gross_area - opening_area).max(0.0)
}

/// Check if an opening shares 2+ points with an edge (collinear check).
fn opening_on_edge(opening: &Opening, edge_a: Uuid, edge_b: Uuid) -> bool {
    let shared = opening.points.iter()
        .filter(|p| **p == edge_a || **p == edge_b)
        .count();
    shared >= 2
}
```

### 3.5 Opening Quantities

```rust
pub fn opening_quantity(unit: UnitType, opening: &Opening) -> f64 {
    match unit {
        UnitType::Piece => 1.0,
        UnitType::SquareMeter => match &opening.kind {
            OpeningKind::Door { height, width } => height * width / 1_000_000.0,
            OpeningKind::Window { height, width, reveal_width, .. } => {
                let reveal_perimeter = 2.0 * height + 2.0 * width;
                reveal_perimeter * reveal_width / 1_000_000.0
            }
        },
        UnitType::LinearMeter => match &opening.kind {
            OpeningKind::Door { height, width } => (2.0 * height + width) / 1000.0,
            OpeningKind::Window { height, width, .. } => {
                (2.0 * height + 2.0 * width) / 1000.0
            }
        },
    }
}
```

## 4. Canvas Rendering

### 4.1 Render Order (Back to Front)

```
1. Grid
2. Room fills (triangulated with earcutr, holes for cutouts)
3. Wall fills (triangulated polygons)
4. Opening fills
5. Edges (lines between points)
6. Points (circles, always on top)
7. Labels (room names, measurements)
```

### 4.2 Hit-Testing Order (Front to Back)

```rust
pub enum HitResult {
    Point(Uuid),
    Edge(Uuid),
    Opening(Uuid),
    Wall(Uuid),
    Room(Uuid),
    Nothing,
}

pub fn hit_test(screen_pos: Pos2, project: &Project, canvas: &CanvasState) -> HitResult {
    // 1. Points: circle hit test, ~8-10px screen radius
    for point in &project.points {
        let screen = canvas.world_to_screen(point.position);
        if screen.distance(screen_pos) < 10.0 {
            return HitResult::Point(point.id);
        }
    }

    // 2. Edges: distance to line segment, ~5px tolerance
    for edge in &project.edges {
        let a = canvas.world_to_screen(project.point(edge.point_a).position);
        let b = canvas.world_to_screen(project.point(edge.point_b).position);
        if distance_to_segment(screen_pos, a, b) < 5.0 {
            return HitResult::Edge(edge.id);
        }
    }

    // 3. Openings: point-in-polygon
    for opening in &project.openings {
        let polygon = opening_screen_polygon(opening, project, canvas);
        if point_in_polygon(screen_pos, &polygon) {
            return HitResult::Opening(opening.id);
        }
    }

    // 4. Walls: point-in-polygon
    for wall in &project.walls {
        let polygon = wall_screen_polygon(wall, project, canvas);
        if point_in_polygon(screen_pos, &polygon) {
            return HitResult::Wall(wall.id);
        }
    }

    // 5. Rooms: point-in-polygon (excluding cutouts)
    for room in &project.rooms {
        let polygon = room_screen_polygon(room, project, canvas);
        if point_in_polygon(screen_pos, &polygon) {
            // Check not inside a cutout
            let in_cutout = room.cutouts.iter().any(|c| {
                let cutout_poly = contour_screen_polygon(c, project, canvas);
                point_in_polygon(screen_pos, &cutout_poly)
            });
            if !in_cutout {
                return HitResult::Room(room.id);
            }
        }
    }

    HitResult::Nothing
}
```

### 4.3 Point Rendering

```rust
fn draw_points(painter: &Painter, project: &Project, canvas: &CanvasState, selection: &Selection) {
    for point in &project.points {
        let screen = canvas.world_to_screen(point.position);
        let radius = 6.0;

        let (fill, stroke) = if selection.is_selected(point.id) {
            (Color32::from_rgb(0, 120, 255), Stroke::new(2.0, Color32::WHITE))
        } else {
            (Color32::from_rgb(200, 200, 200), Stroke::new(1.0, Color32::GRAY))
        };

        painter.circle(screen, radius, fill, stroke);
    }
}
```

### 4.4 Visibility Modes

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisibilityMode {
    /// Everything visible
    All,
    /// Only points and edges (wireframe)
    Wireframe,
    /// Points and rooms (no wall fills)
    Rooms,
}

impl VisibilityMode {
    pub fn show_room_fills(&self) -> bool {
        matches!(self, Self::All | Self::Rooms)
    }
    pub fn show_wall_fills(&self) -> bool {
        matches!(self, Self::All)
    }
    pub fn show_opening_fills(&self) -> bool {
        matches!(self, Self::All)
    }
    // Points and edges are always visible
}
```

## 5. Tools

### 5.1 Tool Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    Point,
    Room,
    Wall,
    Door,
    Window,
}
```

### 5.2 Point Tool

Click on canvas to place a point. Snaps to grid and existing points.

```rust
fn handle_point_tool_click(&mut self, world_pos: DVec2) {
    // Snap to existing point (within threshold)
    if let Some(existing) = self.find_nearest_point(world_pos, SNAP_RADIUS) {
        self.selection.select(existing.id);
        return;
    }

    // Snap to grid
    let snapped = self.snap_to_grid(world_pos);

    // Create new point
    let point = Point {
        id: Uuid::new_v4(),
        position: snapped,
        height: self.project.defaults.wall_height,
    };
    self.history.snapshot(&self.project, "Add point");
    self.project.points.push(point);
}
```

### 5.3 Room Tool

Click points sequentially to define the room contour. Double-click or press Enter to close.

```rust
struct RoomToolState {
    /// Points selected so far for the room contour.
    points: Vec<Uuid>,
    /// Currently building a cutout (after room is created).
    building_cutout: bool,
}

fn handle_room_tool_click(&mut self, world_pos: DVec2) {
    // Find nearest point (must click on existing points)
    let point = match self.find_nearest_point(world_pos, SNAP_RADIUS) {
        Some(p) => p.id,
        None => return, // Must click on a point
    };

    // Avoid duplicate consecutive points
    if self.room_tool.points.last() == Some(&point) {
        return;
    }

    // If clicking the first point again, close the contour
    if self.room_tool.points.len() >= 3 && point == self.room_tool.points[0] {
        self.finalize_room();
        return;
    }

    self.room_tool.points.push(point);
}

fn finalize_room(&mut self) {
    if self.room_tool.points.len() < 3 { return; }

    self.history.snapshot(&self.project, "Add room");

    let room = Room {
        id: Uuid::new_v4(),
        name: format!("Room {}", self.project.rooms.len() + 1),
        points: self.room_tool.points.clone(),
        cutouts: Vec::new(),
    };

    // Ensure edges exist for the contour
    self.project.ensure_contour_edges(&room.points);
    self.project.rooms.push(room);
    self.room_tool.points.clear();
}
```

### 5.4 Wall Tool (Visual)

Similar to room tool — select points to define the wall polygon.

### 5.5 Opening Tools (Door/Window)

Select 4+ points, then set parameters in properties panel.

## 6. Properties Panel

### 6.1 Point Selected

```
Position: X [____] Y [____]
Height:   [____] mm
---
Used in: Room "Kitchen", Room "Hallway", Wall #3
```

### 6.2 Edge Selected

```
Distance: [____] mm  [Reset to computed]
Angle:    [____] deg [Reset to computed]
---
Computed distance: 3,450 mm
Computed angle: 90.2 deg
---
Height at A: 2,700 mm
Height at B: 2,700 mm
Wall area: 9.315 m2
```

### 6.3 Room Selected

```
Name: [____________]
---
Perimeter: 14.200 m
Floor area: 12.540 m2
Wall area: 38.340 m2
---
Edges: 4
Cutouts: 1
---
[Add Cutout]  [Remove Room]
```

## 7. Deleted Modules

The following modules are fully replaced and should be removed:

| Module | Replacement |
|--------|-------------|
| `model/wall.rs` (SideData, SideJunction, SectionData) | `Edge` + `Point.height` |
| `model/room_metrics.rs` | `Room::floor_area()`, `Room::perimeter()` |
| `editor/room_detection.rs` | Manual room creation via Room tool |
| `editor/wall_joints.rs` | Not needed — walls are simple filled polygons |
| `editor/endpoint_merge.rs` | Not needed — points are shared by reference |
| `editor/snap.rs` (T-junction snap) | Simplified: snap to points + snap to grid |

## 8. Migration Path

### 8.1 Strategy

Implement the new model alongside the old one, then switch. No data migration needed — old project files become incompatible (major version bump).

### 8.2 Implementation Order

1. **New model types** (`Point`, `Edge`, `Room`, `Wall`, `Opening` in new `model/` files)
2. **Calculation algorithms** (`floor_area`, `perimeter`, `edge_wall_area`, `opening_quantity`)
3. **Canvas rendering** (render order, point/edge/room/wall/opening drawing)
4. **Hit-testing** (priority-based, front-to-back)
5. **Tools** (Point, Room, Wall, Door, Window)
6. **Properties panel** (editors for each selection type)
7. **Services** (assignment to room edges, openings, rooms)
8. **Export** (Excel report generation from new model)
9. **Persistence** (save/load new format)
10. **Remove old code** (delete replaced modules)

## 9. Design Decisions

1. **Edge direction**: Undirected. An edge is simply "a distance between two points". Direction/winding is determined by the contour that references the edge (e.g., `room.points` order), not by the edge itself. One edge can belong to two rooms with different traversal directions.
2. **Orphan cleanup**: Cascade delete. Deleting a point removes all edges containing that point, and removes all rooms/walls/openings that reference that point (a polygon missing a vertex is invalid). Associated services are also cleaned up.
3. **Room contour winding**: Accept either CW or CCW. Shoelace with `.abs()` handles both. `earcutr` also accepts both. No normalization needed.
4. **Opening width derivation**: Default derived from polygon geometry (distance between shared edge points). User can override with a manual value, same pattern as edge distance overrides.
