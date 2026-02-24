# Key Types

## Model Types (serde-serializable)

### Coordinate Type — `glam::DVec2`

All world-space coordinates use `glam::DVec2` (re-exported from the `glam` crate). All geometry is in millimeters. Free functions in `model/wall.rs`:
- `distance_to_segment(p: DVec2, a: DVec2, b: DVec2) -> f64` — distance from point to line segment
- `project_onto_segment(p: DVec2, a: DVec2, b: DVec2) -> (f64, DVec2)` — project onto segment, returns (t, projected_point)

### `SideJunction` — `src/model/wall.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideJunction {
    pub wall_id: Uuid,  // ID of the connecting wall
    pub t: f64,         // Parametric position along wall (0.0–1.0)
}
```

Represents a T-junction on one side of a wall.

### `SectionData` — `src/model/wall.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionData {
    pub length: f64,        // Section length in mm
    pub height_start: f64,  // Height at section start (mm)
    pub height_end: f64,    // Height at section end (mm)
}
```

Properties of a single wall-side section between junctions.

### `SideData` — `src/model/wall.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideData {
    pub length: f64,                    // Total side length (mm, user-editable)
    pub height_start: f64,              // Height at wall start (mm)
    pub height_end: f64,                // Height at wall end (mm)
    pub junctions: Vec<SideJunction>,   // T-junctions, sorted by t
    pub sections: Vec<SectionData>,     // Section properties (junctions.len()+1 entries)
}
```

Complete data for one side (left or right) of a wall, including junction-split sections.

### `Wall` — `src/model/wall.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    pub start: DVec2,               // Start endpoint (mm)
    pub end: DVec2,                 // End endpoint (mm)
    pub thickness: f64,             // Wall thickness (mm, default 200)
    pub left_side: SideData,        // Left side looking start→end
    pub right_side: SideData,       // Right side looking start→end
    pub openings: Vec<Uuid>,        // IDs of attached openings
}
```

### `OpeningKind` — `src/model/opening.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpeningKind {
    Door {
        height: f64,  // mm (default 2100)
        width: f64,   // mm (default 900)
    },
    Window {
        height: f64,       // mm (default 1400)
        width: f64,        // mm (default 1200)
        sill_height: f64,  // mm (default 900)
        reveal_width: f64, // mm (default 250)
    },
}
```

### `Opening` — `src/model/opening.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    pub kind: OpeningKind,
    pub wall_id: Option<Uuid>,          // Attached wall (None = detached)
    pub offset_along_wall: f64,         // Center offset from wall start (mm)
}
```

Note: `fallback_position` was removed. Detached opening positions are stored transiently in `EditorState.orphan_positions`.

### `Label` — `src/model/label.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: Uuid,
    pub text: String,
    pub position: DVec2,
    pub font_size: f64,   // Display font size (default 14.0)
    pub rotation: f64,    // Rotation in radians (default 0.0)
}
```

### `WallSide` — `src/model/room.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallSide {
    Left,   // Left side looking from wall.start to wall.end
    Right,  // Right side looking from wall.start to wall.end
}
```

### `Room` — `src/model/room.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub name: String,               // Auto-generated "Комната N", user-editable
    pub wall_ids: Vec<Uuid>,        // Ordered wall IDs forming closed contour
    pub wall_sides: Vec<WallSide>,  // Per-wall: which side faces room interior
}
```

### `ProjectDefaults` — `src/model/project.rs`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDefaults {
    pub wall_thickness: f64,       // mm (default 200)
    pub wall_height: f64,          // mm (default 2700)
    pub door_height: f64,          // mm (default 2100)
    pub door_width: f64,           // mm (default 900)
    pub window_height: f64,        // mm (default 1400)
    pub window_width: f64,         // mm (default 1200)
    pub window_sill_height: f64,   // mm (default 900)
    pub window_reveal_width: f64,  // mm (default 250)
}
```

Per-project configurable defaults used when creating new walls, doors, and windows. Set at project creation, editable via project settings window.

### `AssignedService` — `src/model/project.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedService {
    pub service_template_id: Uuid,
    pub custom_price: Option<f64>,  // Override price (None = use template)
}
```

### `SideServices` — `src/model/project.rs`

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SideServices {
    pub sections: Vec<Vec<AssignedService>>,  // One entry per section
}
```

### `WallSideServices` — `src/model/project.rs`

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WallSideServices {
    pub left: SideServices,
    pub right: SideServices,
}
```

### `Project` — `src/model/project.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub rooms: Vec<Room>,
    pub labels: Vec<Label>,
    pub price_list_id: Option<Uuid>,
    pub wall_services: HashMap<Uuid, WallSideServices>,
    pub opening_services: HashMap<Uuid, Vec<AssignedService>>,
    pub room_services: HashMap<Uuid, Vec<AssignedService>>,
    pub defaults: ProjectDefaults,  // #[serde(default)] for backward compat
}
```

Mutation methods on `Project`: `add_wall()`, `remove_wall()`, `add_opening()`, `remove_opening()`, `remove_label()`, `move_opening()`.

Lookup methods: `wall(&self, id)`, `wall_mut(&mut self, id)`, `opening(&self, id)`, `opening_mut(&mut self, id)`, `room(&self, id)`.

### `UnitType` — `src/model/price.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitType {
    Piece,        // шт.
    SquareMeter,  // м²
    LinearMeter,  // п.м.
}
```

### `TargetObjectType` — `src/model/price.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetObjectType {
    Wall,
    Window,
    Door,
    Room,
}
```

### `ServiceTemplate` — `src/model/price.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTemplate {
    pub id: Uuid,
    pub name: String,
    pub unit_type: UnitType,
    pub price_per_unit: f64,
    pub target_type: TargetObjectType,
}
```

### `PriceList` — `src/model/price.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceList {
    pub id: Uuid,
    pub name: String,
    pub services: Vec<ServiceTemplate>,
}
```

---

## Editor State Types

### `EditorTool` — `src/editor/mod.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Wall,
    Door,
    Window,
    Label,
}
```

Hotkeys: V (Select), W (Wall), D (Door), O (Window), T (Label).

### `Selection` — `src/editor/mod.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    None,
    Wall(Uuid),
    Opening(Uuid),
    Room(Uuid),
    Label(Uuid),
}
```

### `EditorState` — `src/editor/mod.rs`

```rust
pub struct EditorState {
    pub active_tool: EditorTool,
    pub selection: Selection,
    pub canvas: Canvas,
    pub wall_tool: WallTool,
    pub opening_tool: OpeningTool,
    pub orphan_positions: HashMap<Uuid, DVec2>,  // Transient: world pos for detached openings
}
```

### `Canvas` — `src/editor/canvas.rs`

```rust
pub struct Canvas {
    pub offset: egui::Vec2,           // Pan offset in world mm
    pub zoom: f32,                    // Pixels per mm (0.02–5.0, default 0.5)
    pub grid_step: f64,               // Grid step in mm (100.0)
    pub cursor_world_pos: Option<egui::Pos2>,  // Current cursor in world mm
}
```

### `WallToolState` — `src/editor/wall_tool.rs`

```rust
#[derive(Debug, Clone)]
pub enum WallToolState {
    Idle,
    Drawing { start: DVec2 },
}
```

### `WallTool` — `src/editor/wall_tool.rs`

```rust
pub struct WallTool {
    pub state: WallToolState,
    pub preview_end: Option<DVec2>,
    pub chain_start: Option<DVec2>,
    pub last_snap: Option<SnapResult>,
    pub start_snap: Option<SnapResult>,
    pub chain_start_snap: Option<SnapResult>,
}
```

### `OpeningTool` — `src/editor/mod.rs`

```rust
pub struct OpeningTool {
    pub hover_wall_id: Option<Uuid>,
    pub hover_offset: f64,
}
```

### `SnapType` — `src/editor/snap.rs`

```rust
#[derive(Debug, Clone)]
pub enum SnapType {
    None,           // Shift held — free drawing
    Grid,           // Snapped to nearest grid intersection
    Vertex,         // Snapped to existing wall endpoint
    WallEdge {      // Snapped to wall side edge (T-junction)
        wall_id: Uuid,
        side: WallSide,
        t: f64,
    },
}
```

### `SnapResult` — `src/editor/snap.rs`

```rust
#[derive(Debug, Clone)]
pub struct SnapResult {
    pub position: DVec2,
    pub snap_type: SnapType,
}
```

### `RoomMetrics` — `src/model/room_metrics.rs`

```rust
pub struct RoomMetrics {
    pub inner_polygon: Vec<DVec2>,
    pub gross_area: f64,   // mm² (centerline polygon)
    pub net_area: f64,     // mm² (interior polygon; guarded against self-intersection)
    pub perimeter: f64,    // mm (sum of room-facing side section lengths)
}
```

---

## Room Detection Types

### `GraphVertex` — `src/editor/room_detection.rs`

```rust
pub struct GraphVertex {
    pub position: DVec2,
    pub edges: Vec<(usize, Uuid, f64)>,  // (neighbor_idx, wall_id, angle_radians)
}
```

### `WallGraph` — `src/editor/room_detection.rs`

```rust
pub struct WallGraph {
    pub vertices: Vec<GraphVertex>,
}
```

### `DirectedEdge` — `src/editor/room_detection.rs`

```rust
pub struct DirectedEdge {
    pub from: usize,
    pub to: usize,
    pub wall_id: Uuid,
}
```

---

## Wall Joint Rendering Types

### `JointVertices` — `src/editor/wall_joints.rs`

```rust
pub struct JointVertices {
    pub left: DVec2,   // World-space (mm)
    pub right: DVec2,  // World-space (mm)
}
```

### `HubPolygon` — `src/editor/wall_joints.rs`

```rust
pub struct HubPolygon {
    pub vertices: Vec<DVec2>,  // World-space (mm)
    pub fill: egui::Color32,
}
```

### `WallAtJunction` — `src/editor/wall_joints.rs` (private)

```rust
struct WallAtJunction {
    wall_id: Uuid,
    is_end: bool,
    angle: f64,
    half_thick: f64,
    left: DVec2,   // World-space (mm)
    right: DVec2,  // World-space (mm)
    dir: DVec2,
}
```

---

## History Type

### `History` — `src/app/history.rs`

```rust
pub struct History {
    undo_stack: VecDeque<(Project, &'static str)>,
    redo_stack: VecDeque<(Project, &'static str)>,
    pub version: u64,      // Monotonically increasing, bumped on snapshot/undo/redo/mark_dirty
    max_entries: usize,    // 100
}
```

Snapshot-based undo/redo. `snapshot()` clones the entire `Project` before mutation. `undo()`/`redo()` swap the whole project state. `mark_dirty()` bumps version without storing a snapshot (for non-undoable changes like service edits).

---

## UI Types (private to app/)

### `AppScreen` — `src/app/mod.rs`

```rust
enum AppScreen {
    ProjectList,  // Startup screen
    Editor,       // Main editor
}
```

### `ServiceTarget` — `src/app/mod.rs`

```rust
enum ServiceTarget {
    WallSide { wall_id: Uuid, side: WallSide, section_index: usize },
    Opening { opening_id: Uuid },
    Room { room_id: Uuid },
}
```

### `App` — `src/app/mod.rs`

See [06_STATE_MANAGEMENT.md](06_STATE_MANAGEMENT.md) for field breakdown.

---

## Persistence Types

### `ProjectEntry` — `src/persistence.rs`

```rust
pub struct ProjectEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}
```

### `AssignedServiceRow` — `src/app/services_panel.rs` (pub(super))

```rust
pub(super) struct AssignedServiceRow {
    pub name: String,
    pub unit_label: String,
    pub template_price: f64,
    pub effective_price: f64,
    pub has_custom: bool,
    pub qty: f64,
    pub valid: bool,
}
```
