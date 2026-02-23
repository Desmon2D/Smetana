# Key Types

## Model Types (serde-serializable)

### `Point2D` — `src/model/wall.rs:29`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f64,  // World X in mm
    pub y: f64,  // World Y in mm
}
```

Used everywhere for world-space coordinates. All geometry is in millimeters.

### `SideJunction` — `src/model/wall.rs:6`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideJunction {
    pub wall_id: Uuid,  // ID of the connecting wall
    pub t: f64,         // Parametric position along wall (0.0–1.0)
}
```

Represents a T-junction on one side of a wall.

### `SectionData` — `src/model/wall.rs:14`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionData {
    pub length: f64,        // Section length in mm
    pub height_start: f64,  // Height at section start (mm)
    pub height_end: f64,    // Height at section end (mm)
}
```

Properties of a single wall-side section between junctions.

### `SideData` — `src/model/wall.rs:68`

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

### `Wall` — `src/model/wall.rs:171`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    pub start: Point2D,             // Start endpoint (mm)
    pub end: Point2D,               // End endpoint (mm)
    pub thickness: f64,             // Wall thickness (mm, default 200)
    pub left_side: SideData,        // Left side looking start→end
    pub right_side: SideData,       // Right side looking start→end
    pub openings: Vec<Uuid>,        // IDs of attached openings
}
```

### `OpeningKind` — `src/model/opening.rs:8`

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

### `Opening` — `src/model/opening.rs:66`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    pub kind: OpeningKind,
    pub wall_id: Option<Uuid>,          // Attached wall (None = detached)
    pub offset_along_wall: f64,         // Center offset from wall start (mm)
    pub fallback_position: Option<Point2D>,  // World pos when detached
}
```

### `WallSide` — `src/model/room.rs:6`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallSide {
    Left,   // Left side looking from wall.start to wall.end
    Right,  // Right side looking from wall.start to wall.end
}
```

### `Room` — `src/model/room.rs:13`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub name: String,               // Auto-generated "Комната N", user-editable
    pub wall_ids: Vec<Uuid>,        // Ordered wall IDs forming closed contour
    pub wall_sides: Vec<WallSide>,  // Per-wall: which side faces room interior
}
```

### `ProjectDefaults` — `src/model/project.rs:9`

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

### `AssignedService` — `src/model/project.rs:43`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedService {
    pub service_template_id: Uuid,
    pub custom_price: Option<f64>,  // Override price (None = use template)
}
```

### `SideServices` — `src/model/project.rs:17`

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SideServices {
    pub sections: Vec<Vec<AssignedService>>,  // One entry per section
}
```

### `WallSideServices` — `src/model/project.rs:43`

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WallSideServices {
    pub left: SideServices,
    pub right: SideServices,
}
```

### `Project` — `src/model/project.rs:83`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub rooms: Vec<Room>,
    pub price_list_id: Option<Uuid>,
    pub wall_services: HashMap<Uuid, WallSideServices>,
    pub opening_services: HashMap<Uuid, Vec<AssignedService>>,
    pub room_services: HashMap<Uuid, Vec<AssignedService>>,
    pub defaults: ProjectDefaults,  // #[serde(default)] for backward compat
}
```

### `UnitType` — `src/model/price.rs:5`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitType {
    Piece,        // шт.
    SquareMeter,  // м²
    LinearMeter,  // п.м.
}
```

### `TargetObjectType` — `src/model/price.rs:28`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetObjectType {
    Wall,
    Window,
    Door,
    Room,
}
```

### `ServiceTemplate` — `src/model/price.rs:55`

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

### `PriceList` — `src/model/price.rs:81`

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

### `EditorTool` — `src/editor/mod.rs:21`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Wall,
    Door,
    Window,
}
```

Hotkeys: V (Select), W (Wall), D (Door), O (Window).

### `Selection` — `src/editor/mod.rs:30`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    None,
    Wall(Uuid),
    Opening(Uuid),
    Room(Uuid),
}
```

### `EditorState` — `src/editor/mod.rs:38`

```rust
pub struct EditorState {
    pub active_tool: EditorTool,
    pub selection: Selection,
    pub canvas: Canvas,
    pub wall_tool: WallTool,
    pub opening_tool: OpeningTool,
}
```

### `Canvas` — `src/editor/canvas.rs:7`

```rust
pub struct Canvas {
    pub offset: egui::Vec2,           // Pan offset in world mm
    pub zoom: f32,                    // Pixels per mm (0.02–5.0, default 0.5)
    pub grid_step: f64,               // Grid step in mm (100.0)
    pub cursor_world_pos: Option<egui::Pos2>,  // Current cursor in world mm
}
```

### `WallToolState` — `src/editor/wall_tool.rs:5`

```rust
#[derive(Debug, Clone)]
pub enum WallToolState {
    Idle,
    Drawing { start: Point2D },
}
```

### `WallTool` — `src/editor/wall_tool.rs:14`

```rust
pub struct WallTool {
    pub state: WallToolState,
    pub preview_end: Option<Point2D>,
    pub chain_start: Option<Point2D>,
    pub last_snap: Option<SnapResult>,
    pub start_snap: Option<SnapResult>,
    pub chain_start_snap: Option<SnapResult>,
}
```

### `OpeningTool` — `src/editor/opening_tool.rs:7`

```rust
pub struct OpeningTool {
    pub hover_wall_id: Option<Uuid>,
    pub hover_offset: f64,
}
```

### `SnapType` — `src/editor/snap.rs:9`

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

### `SnapResult` — `src/editor/snap.rs:26`

```rust
#[derive(Debug, Clone)]
pub struct SnapResult {
    pub position: Point2D,
    pub snap_type: SnapType,
}
```

### `RoomMetrics` — `src/editor/room_metrics.rs:5`

```rust
pub struct RoomMetrics {
    pub inner_polygon: Vec<Point2D>,
    pub gross_area: f64,   // mm² (centerline polygon)
    pub net_area: f64,     // mm² (interior polygon minus columns)
    pub perimeter: f64,    // mm (sum of room-facing side section lengths)
}
```

---

## Room Detection Types

### `GraphVertex` — `src/editor/room_detection.rs:10`

```rust
pub struct GraphVertex {
    pub position: Point2D,
    pub edges: Vec<(usize, Uuid, f64)>,  // (neighbor_idx, wall_id, angle_radians)
}
```

### `WallGraph` — `src/editor/room_detection.rs:19`

```rust
pub struct WallGraph {
    pub vertices: Vec<GraphVertex>,
}
```

### `DirectedEdge` — `src/editor/room_detection.rs:315`

```rust
pub struct DirectedEdge {
    pub from: usize,
    pub to: usize,
    pub wall_id: Uuid,
}
```

---

## Wall Joint Rendering Types

### `JointVertices` — `src/editor/wall_joints.rs:17`

```rust
pub struct JointVertices {
    pub left: egui::Pos2,
    pub right: egui::Pos2,
}
```

### `HubPolygon` — `src/editor/wall_joints.rs:24`

```rust
pub struct HubPolygon {
    pub vertices: Vec<egui::Pos2>,
    pub fill: egui::Color32,
}
```

### `WallAtJunction` — `src/editor/wall_joints.rs:31` (pub(super))

```rust
pub(super) struct WallAtJunction {
    pub(super) wall_id: Uuid,
    pub(super) is_end: bool,
    pub(super) angle: f32,
    pub(super) half_thick: f32,
    pub(super) left: egui::Pos2,
    pub(super) right: egui::Pos2,
    pub(super) dir: egui::Vec2,
}
```

---

## History / Command Types

### `Command` trait — `src/history.rs:3`

```rust
pub trait Command {
    fn execute(&mut self, project: &mut Project);
    fn undo(&mut self, project: &mut Project);
    fn description(&self) -> &str;
}
```

### `WallProps` — `src/history.rs:173`

```rust
#[derive(Clone)]
pub struct WallProps {
    pub thickness: f64,
    pub left_side: SideData,
    pub right_side: SideData,
}
```

Snapshot of wall properties for undo/redo of property edits.

### `History` — `src/history.rs:308`

```rust
pub struct History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    pub version: u64,  // Monotonically increasing, bumped on push/undo/redo
}
```

### Command Variants

| Struct | File:Line | Mutates |
|--------|-----------|---------|
| `AddWallCommand` | `history.rs:11` | Adds wall to `project.walls`, optionally adds junctions to target walls' sides at both the start and end points |
| `RemoveWallCommand` | `history.rs:75` | Removes wall, detaches openings (sets fallback_position), removes junctions on other walls |
| `ModifyWallCommand` | `history.rs:166` | Changes wall thickness, left_side, right_side |
| `AddOpeningCommand` | `history.rs:209` | Adds opening to `project.openings`, links to wall's `openings` list |
| `RemoveOpeningCommand` | `history.rs:239` | Removes opening from project, unlinks from wall |
| `ModifyOpeningCommand` | `history.rs:276` | Changes opening kind (dimensions) |

---

## UI Types (private to app/)

### `AppScreen` — `src/app/mod.rs:19`

```rust
enum AppScreen {
    ProjectList,  // Startup screen
    Editor,       // Main editor
}
```

### `ServiceTarget` — `src/app/mod.rs:26`

```rust
enum ServiceTarget {
    WallSide { wall_id: Uuid, side: WallSide, section_index: usize },
    Opening { opening_id: Uuid },
    Room { room_id: Uuid },
}
```

### `App` — `src/app/mod.rs:32`

See [06_STATE_MANAGEMENT.md](06_STATE_MANAGEMENT.md) for field breakdown.

---

## Persistence Types

### `ProjectEntry` — `src/persistence/project_io.rs:10`

```rust
pub struct ProjectEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}
```

### `AssignedServiceRow` — `src/app/services_panel.rs:7` (pub(super))

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
