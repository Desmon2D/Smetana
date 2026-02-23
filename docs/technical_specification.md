# Technical Specification: Construction Estimate Application

## 1. General Description

A desktop application built in Rust with a graphical 2D floor plan editor for calculating construction estimates. The application allows drawing walls, placing windows and doors on them, automatically detecting rooms from closed contours, assigning services from a price list, and generating an Excel report.

**Interface language:** Russian.  
**Target environment:** low-end hardware (Windows).  
**GUI framework:** egui/eframe (immediate mode, minimal overhead, native rendering).

---

## 2. Architecture

### 2.1 Module Structure

```
construction-estimator/
├── src/
│   ├── main.rs                  # Entry point, eframe initialization
│   ├── app.rs                   # Main App struct, UI routing
│   ├── model/
│   │   ├── mod.rs
│   │   ├── wall.rs              # Wall: points, thickness, heights
│   │   ├── opening.rs           # Windows and doors
│   │   ├── room.rs              # Room (auto-detected contour)
│   │   ├── project.rs           # Project (walls, windows, doors, rooms, services)
│   │   └── price.rs             # Service price list
│   ├── editor/
│   │   ├── mod.rs
│   │   ├── canvas.rs            # 2D canvas with grid, pan/zoom
│   │   ├── wall_tool.rs         # Wall drawing tool
│   │   ├── opening_tool.rs      # Window/door placement tool
│   │   ├── select_tool.rs       # Selection, movement, deletion
│   │   ├── snap.rs              # Snap system for vertices/grid
│   │   └── room_detection.rs    # Automatic room detection
│   ├── panels/
│   │   ├── mod.rs
│   │   ├── properties.rs        # Properties panel for selected object
│   │   ├── rooms.rs             # Room list and settings
│   │   ├── price_panel.rs       # Price list editor
│   │   └── services_panel.rs    # Service assignment to objects
│   ├── export/
│   │   ├── mod.rs
│   │   └── excel.rs             # .xlsx report generation
│   ├── persistence/
│   │   ├── mod.rs
│   │   ├── project_io.rs        # Project save/load (JSON)
│   │   └── price_io.rs          # Price list save/load (JSON)
│   └── history.rs               # Undo/Redo
├── saves/                       # Saves folder
│   ├── projects/                # Project JSON files
│   └── prices/                  # Price list JSON files
└── Cargo.toml
```

### 2.2 Key Dependencies (Cargo.toml)

| Crate | Purpose |
|-------|---------|
| `eframe` / `egui` | GUI framework |
| `serde` + `serde_json` | Model serialization |
| `rust_xlsxwriter` | .xlsx generation |
| `uuid` | Unique object IDs |
| `rfd` | Native file dialogs |

---

## 3. Data Model

### 3.1 Wall

```rust
struct Wall {
    id: Uuid,
    /// Start point in world coordinates (mm)
    start: Point2D,
    /// End point in world coordinates (mm)
    end: Point2D,
    /// Wall thickness (mm)
    thickness: f64,
    /// Height at the start edge (mm)
    height_start: f64,
    /// Height at the end edge (mm)
    height_end: f64,
    /// Attached openings
    openings: Vec<Uuid>,
}
```

**Canvas rendering:** rectangle (not a trapezoid). The wall is drawn as a line with thickness. The actual shape (trapezoid due to different heights) is only considered in area calculations.

**Wall area (trapezoid):**
```
S = length × (height_start + height_end) / 2
```

### 3.2 Opening

```rust
enum OpeningKind {
    Door {
        height: f64,      // mm
        width: f64,       // mm
        // depth = wall thickness (automatic)
    },
    Window {
        height: f64,       // mm
        width: f64,        // mm
        sill_height: f64,  // height from floor (mm)
        reveal_width: f64, // reveal width (mm)
    },
}

struct Opening {
    id: Uuid,
    kind: OpeningKind,
    /// ID of the wall it is attached to (None = not attached, validation error)
    wall_id: Option<Uuid>,
    /// Offset from wall start to the center of the opening (mm)
    offset_along_wall: f64,
    /// Assigned services
    assigned_services: Vec<AssignedService>,
}
```

**Validation:** if `wall_id == None`, the opening is highlighted in red. The "Generate Report" button is disabled.

### 3.3 Room

```rust
struct Room {
    id: Uuid,
    name: String,  // default "Room N"
    /// Ordered list of wall IDs forming a closed contour
    wall_ids: Vec<Uuid>,
    /// For each wall — which side faces the room interior
    wall_sides: Vec<WallSide>,  // Inner / Outer
    /// Assigned services (for floor, ceiling, etc.)
    assigned_services: Vec<AssignedService>,
}
```

**Automatic detection:** the algorithm for finding minimum closed cycles in the wall graph runs on every wall change. Partition walls belong to two rooms.

**Floor area:** calculated from the inner edges of walls (accounting for thickness). Uses the Shoelace formula for arbitrary polygons.

### 3.4 Price List (PriceList)

```rust
enum UnitType {
    Piece,          // per piece
    SquareMeter,    // per m²
    LinearMeter,    // per linear meter
}

enum TargetObjectType {
    Wall,
    Window,
    Door,
    Room,  // floor, ceiling, etc.
}

struct ServiceTemplate {
    id: Uuid,
    name: String,                  // "Wall plastering"
    unit_type: UnitType,
    price_per_unit: f64,           // price per unit (rubles)
    target_type: TargetObjectType, // which object type it applies to
}

struct PriceList {
    name: String,
    services: Vec<ServiceTemplate>,
}
```

### 3.5 Assigned Service (AssignedService)

```rust
struct AssignedService {
    service_template_id: Uuid,
    /// Overridden price (if None — taken from template)
    custom_price: Option<f64>,
}
```

### 3.6 Project

```rust
struct Project {
    id: Uuid,
    name: String,
    walls: Vec<Wall>,
    openings: Vec<Opening>,
    rooms: Vec<Room>,
    /// ID of the price list in use
    price_list_id: Option<Uuid>,
    /// Assigned services by object
    wall_services: HashMap<Uuid, Vec<AssignedService>>,
    opening_services: HashMap<Uuid, Vec<AssignedService>>,
    room_services: HashMap<Uuid, Vec<AssignedService>>,
}
```

---

## 4. Graphical Editor

### 4.1 Canvas

- 2D top-down view, infinite panning with the middle mouse button or holding Space + LMB.
- Zoom with the mouse wheel.
- Grid with step size (default 100 mm). Displayed with adaptive density depending on zoom level.
- Coordinates in millimeters, displayed in the status bar.

### 4.2 Wall Tool

- User clicks the start point → moves the mouse → clicks the end point.
- Sequential clicks continue the wall chain.
- Double-click or Esc finishes the chain.
- Clicking the starting vertex of the chain closes the contour.

**Snapping:**
- To existing vertices (15px screen radius).
- To the grid.
- Holding Shift → snapping is disabled (free drawing).

**Wall properties (right panel):**
- Thickness (mm), default 200.
- Height at start edge (mm), default 2700.
- Height at end edge (mm), default 2700.
- Length (auto-calculated, read-only).

### 4.3 Door / Window Tool

- User selects the opening type from the toolbar.
- Drag-and-drop from the side panel or toolbar onto the canvas.
- When dragging over a wall — the opening "snaps" to the wall and is displayed on it.
- When released outside a wall — the opening remains red (validation error).
- A placed opening can be dragged along the wall or to another wall.

**Door properties (right panel):**
- Height (mm), default 2100.
- Width (mm), default 900.
- Depth (read-only, = wall thickness).

**Window properties (right panel):**
- Height (mm), default 1400.
- Width (mm), default 1200.
- Sill height (mm), default 900.
- Reveal width (mm), default 250.

### 4.4 Selection Tool

- Click on an object — select it. Properties are shown in the right panel.
- Delete — remove the selected object.
- Drag — move (for walls — move vertices).

### 4.5 Automatic Room Detection

The algorithm runs on every change to wall topology:

1. Build a planar graph from vertices and edges (walls).
2. For each edge, find the minimum closed cycles (minimum angle traversal algorithm — Minimum Cycle Basis, or using a half-edge structure).
3. Each cycle = a room.
4. For each wall in the cycle, determine which side faces the room interior.
5. The outer contour (largest cycle) is not a room.

Rooms are displayed on the canvas with semi-transparent fills in different colors. The room name is displayed at the center.

---

## 5. Interface Panels

### 5.1 Top Panel (Toolbar)

- Tool selection: Selection | Wall | Door | Window.
- Buttons: New Project | Open | Save | Undo | Redo.
- "Generate Report" button (active only when there are no validation errors).

### 5.2 Left Panel — Project Structure

- Tree view: Rooms → Walls → Openings.
- Clicking an item = selecting it on the canvas.

### 5.3 Right Panel — Properties

- Shows properties of the selected object (wall / window / door / room).
- For a room: name, floor area, perimeter.
- Editable fields with immediate application.

### 5.4 Bottom Panel — Price List and Services

#### "Price List" Tab
- Services table: name, object type, unit of measurement, price per unit.
- Buttons: Add | Delete | Import Price List | Export Price List.

#### "Assigned Services" Tab
- For the selected object: list of assigned services.
- Drag-and-drop from the price list or "Add Service" button.
- Display of calculated quantity and cost.

---

## 6. Calculations

### 6.1 Wall

| Parameter | Formula |
|-----------|---------|
| Length | `distance(start, end)` |
| Area (gross) | `length × (height_start + height_end) / 2` |
| Openings area | `Σ (width × height)` for all doors and windows on the wall |
| Area (net) | Gross area − Openings area |

### 6.2 Window

| Parameter | Formula |
|-----------|---------|
| Reveal perimeter | `2 × height + 2 × width` (if reveal runs along all 4 sides) or `2 × height + width` (if there's a windowsill at the bottom, no reveal) — **clarification: calculated along all 4 sides** |
| Reveal area | `reveal_perimeter × reveal_width` |

### 6.3 Door

| Parameter | Formula |
|-----------|---------|
| Depth | = wall thickness |
| Perimeter | `2 × height + width` (no threshold) |
| Opening area | `height × width` |
| Belonging | Determined by the two rooms adjacent to the wall |

### 6.4 Room

| Parameter | Formula |
|-----------|---------|
| Floor area | Shoelace formula on the interior vertices of the contour |
| Perimeter | Sum of lengths of the inner wall edges |
| Total wall area (gross) | Sum of areas of all room walls |
| Total wall area (net) | Gross − areas of all openings |

### 6.5 Estimate

For each assigned service:

| Unit | Quantity |
|------|----------|
| Piece | 1 |
| m² | Object area (net wall, floor, reveal, etc.) |
| lin. m. | Object perimeter |

**Service total** = quantity × price per unit.

---

## 7. Excel Report (.xlsx)

### Sheet 1: "Rooms"

A unified table for all rooms:

| Room | Floor Area (m²) | Perimeter (m) | Gross Wall Area (m²) | Net Wall Area (m²) |
|------|-----------------|----------------|----------------------|---------------------|
| Kitchen | 12.5 | 14.2 | 38.3 | 34.1 |
| Bedroom | 18.0 | 17.0 | 45.9 | 42.2 |

Followed by a detailed breakdown for each room:

**Room: Kitchen**

*Walls:*

| Wall | Start Height (mm) | End Height (mm) | Length (mm) | Thickness (mm) | Gross Area (m²) | Net Area (m²) |
|------|-------------------|-----------------|-------------|-----------------|------------------|----------------|
| W1 | 2700 | 2700 | 4000 | 200 | 10.80 | 9.12 |

*Windows:*

| Window | Height (mm) | Width (mm) | Reveal (mm) | Sill Height (mm) | Reveal Perimeter (m) | Reveal Area (m²) |
|--------|-------------|------------|-------------|-------------------|----------------------|-------------------|
| W1 | 1400 | 1200 | 250 | 900 | 5.20 | 1.30 |

### Sheet 2: "Doors"

| Door | Height (mm) | Width (mm) | Depth (mm) | Perimeter (m) | From Room | To Room |
|------|-------------|------------|------------|----------------|-----------|---------|
| D1 | 2100 | 900 | 200 | 5.10 | Kitchen | Hallway |

### Sheet 3: "Estimate"

| Room/Object | Service | Unit | Quantity | Price per Unit (₽) | Cost (₽) |
|-------------|---------|------|----------|---------------------|-----------|
| Kitchen / Wall W1 | Plastering | m² | 9.12 | 500 | 4,560 |
| Kitchen / Window W1 | Reveals | m² | 1.30 | 800 | 1,040 |
| Door D1 | Installation | pcs | 1 | 3,000 | 3,000 |
| | | | | **TOTAL:** | **8,600** |

---

## 8. Save and Load

### 8.1 Project → JSON

File: `saves/projects/{project_name}.json`

Contains all project data: walls, openings, rooms (with names), assigned services.

### 8.2 Price List → JSON

File: `saves/prices/{price_list_name}.json`

Contains service templates. Price lists are independent of projects and can be reused.

### 8.3 Auto-save

The project is auto-saved on every significant action (adding/removing/modifying an object).

---

## 9. Undo/Redo

Implemented using the Command pattern:

```rust
trait Command {
    fn execute(&mut self, project: &mut Project);
    fn undo(&mut self, project: &mut Project);
    fn description(&self) -> &str;
}

struct History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
}
```

Keyboard shortcuts: Ctrl+Z (undo), Ctrl+Y or Ctrl+Shift+Z (redo).

---

## 10. Keyboard Shortcuts

| Key | Action |
|-----|--------|
| V | Selection tool |
| W | Wall tool |
| D | Door tool |
| O | Window tool |
| Delete | Delete selected object |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+S | Save project |
| Ctrl+N | New project |
| Ctrl+O | Open project |
| Escape | Cancel current action / deselect |
| Shift (held) | Disable snapping |

---

## 11. Limitations and Assumptions

1. The application works with a single floor only (2D plan).
2. Walls are straight lines (no arcs/curves).
3. All dimensions are entered in millimeters; the report outputs values in mm and m².
4. Window reveal is calculated along all 4 sides (including the bottom).
5. Door perimeter is calculated without a threshold (2 × height + width).
6. A wall that serves as a partition is included in both rooms.
7. Graphical wall rendering is rectangular (the actual trapezoid is only used in calculations).
