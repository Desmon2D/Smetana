# Implementation Plan: Point-First Redesign

This document breaks the redesign into sequential LLM sessions. Each session is
a self-contained unit of work for one conversation. Sessions must be executed in
order — each builds on the previous.

**Current codebase:** ~6,300 lines across model/, editor/, app/, export/,
persistence.rs, main.rs.

**Target:** Replace the wall-first data model with a point-first model (see
`01_CONCEPT.md` and `02_TECHNICAL.md`), and strip services/export for later
reimplementation.

---

## Session 1 — Remove Services, Price List, and Export

**Goal:** Strip all service assignment, price list management, quantity
computation, and Excel export functionality. The app should compile and run
normally afterward — geometry editing, rendering, and persistence all still work.

### Files to DELETE

| File | Reason |
|------|--------|
| `src/app/services_panel.rs` | Service display UI |
| `src/app/service_picker.rs` | Service assignment dialog |
| `src/app/price_list.rs` | Price list CRUD window |
| `src/export/excel.rs` | Excel workbook generation |
| `src/export/excel_sheets.rs` | Per-sheet writers |
| `src/export/mod.rs` | Module root |
| `src/model/price.rs` | PriceList, ServiceTemplate, UnitType, TargetObjectType |
| `src/model/quantity.rs` | Quantity computation functions |

### Files to MODIFY

**`Cargo.toml`**
- Remove `rust_xlsxwriter` dependency
- Remove `chrono` dependency

**`src/model/mod.rs`**
- Remove `pub mod price;` and `pub mod quantity;`
- Remove `pub use price::*;` and `pub use quantity::*;`

**`src/model/project.rs`**
- Delete `AssignedService`, `SideServices`, `WallSideServices` structs
- Remove from `Project`: `price_list_id`, `wall_services`, `opening_services`,
  `room_services`
- Update `Project::new()` accordingly

**`src/app/mod.rs`**
- Remove module declarations: `mod price_list;`, `mod service_picker;`,
  `mod services_panel;`
- Delete `ServiceTarget` enum
- Remove from `App` struct: `price_list`, `selected_service_idx`,
  `show_price_list_window`, `show_service_picker`, `service_picker_filter`,
  `service_picker_target`, `price_list_filter`
- Update `App::new()` — remove initializers for deleted fields
- In `eframe::App::update()` — remove `show_price_list_window_ui()` and
  `show_service_picker_window()` calls
- In `merge_rooms()` — remove `room_services` cleanup (lines ~188–195)

**`src/app/toolbar.rs`**
- Remove `use crate::export::export_to_xlsx;`
- Remove the "Сформировать отчёт" button block (lines ~138–159)
- Remove the "Услуги" button (lines ~163–165)
- Delete `has_validation_errors()` if it exists only for export gating

**`src/app/properties_panel.rs`**
- Remove `use crate::model::{TargetObjectType, ...section_net_area}` — keep
  only geometry imports
- In `show_wall_properties()` — remove the "Услуги" section at the bottom
  (service display and `show_wall_side_services` calls)
- In `show_opening_properties()` — remove the "Услуги" section
  (`opening_services`, `build_assigned_rows_for`, `show_flat_services`)
- In `show_room_properties()` — remove the "Услуги" section
  (`room_services`, `build_assigned_rows_for`, `show_flat_services`)
- Remove `opening_errors()` and `selection_target_type()` if they only serve
  service validation

**`src/persistence.rs`**
- Remove all price list I/O: `price_path()`, `save_price_list()`,
  `save_price_list_to()`, `load_price_list()`
- Remove `PRICES_DIR` constant
- Update `ensure_saves_dirs()` — only create projects dir
- Remove `round_trip_price_list` test
- Update `round_trip_project_with_wall` test — remove any service-related
  assertions
- Remove `use crate::model::{ServiceTemplate, TargetObjectType, UnitType}`
  from test imports

### Verification

```bash
cargo build        # Must compile cleanly
cargo test         # round_trip_project_with_wall passes
cargo clippy       # No warnings from removed code
cargo run          # App launches, geometry editing works, no service/export UI
```

---

## Session 2 — New Model Types

**Goal:** Replace all model types with the point-first data model. After this
session the model layer compiles, but the rest of the app does NOT — that is
expected. Broken references in editor/ and app/ will be fixed in later sessions.

### Files to DELETE

| File | Reason |
|------|--------|
| `src/model/wall.rs` | Old Wall (centerline, sides, junctions, sections) |
| `src/model/opening.rs` | Old Opening (wall_id, offset_along_wall) |
| `src/model/room.rs` | Old Room (wall_ids, wall_sides) |
| `src/model/label.rs` | Labels will be preserved but folded into project.rs |
| `src/model/room_metrics.rs` | Replaced by Room methods |

### Files to CREATE

**`src/model/point.rs`**
```rust
pub struct Point {
    pub id: Uuid,
    pub position: DVec2,   // mm, world coordinates
    pub height: f64,       // mm, ceiling height at this point
}
```

**`src/model/edge.rs`**
```rust
pub struct Edge {
    pub id: Uuid,
    pub point_a: Uuid,
    pub point_b: Uuid,
    pub distance_override: Option<f64>,  // mm
    pub angle_override: Option<f64>,     // degrees
}
```
Methods: `distance(&self, points)`, `angle(&self, prev_edge, points)`.
Free function: `compute_angle_from_coords()`.
Free function: `shoelace_area(polygon: &[DVec2]) -> f64`.

**`src/model/room.rs`** (new)
```rust
pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub points: Vec<Uuid>,          // outer contour
    pub cutouts: Vec<Vec<Uuid>>,    // inner contours
}
```
Methods: `floor_area(&self, project)`, `perimeter(&self, project)`.
Private: `contour_area()`, `area_from_coordinates()`,
`area_from_measurements()`.

**`src/model/wall.rs`** (new — visual polygon)
```rust
pub struct Wall {
    pub id: Uuid,
    pub points: Vec<Uuid>,
    pub color: [u8; 4],
}
```

**`src/model/opening.rs`** (new — polygon-based)
```rust
pub struct Opening {
    pub id: Uuid,
    pub points: Vec<Uuid>,
    pub kind: OpeningKind,
}

pub enum OpeningKind {
    Door { height: f64, width: f64 },
    Window { height: f64, width: f64, sill_height: f64, reveal_width: f64 },
}
```
Keep `OpeningKind::width()`, `height()`, `target_type()` methods.

### Files to MODIFY

**`src/model/mod.rs`**
- Replace module list: `point`, `edge`, `room`, `wall`, `opening`, `project`
- Update re-exports

**`src/model/project.rs`** (major rewrite)
```rust
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub rooms: Vec<Room>,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub labels: Vec<Label>,      // preserved from old model
    pub defaults: ProjectDefaults,
}
```

New `ProjectDefaults`:
```rust
pub struct ProjectDefaults {
    pub point_height: f64,       // default height for new points (was wall_height)
    pub door_height: f64,
    pub door_width: f64,
    pub window_height: f64,
    pub window_width: f64,
    pub window_sill_height: f64,
    pub window_reveal_width: f64,
}
```

Methods to implement:
- `point(id)`, `point_mut(id)`, `edge(id)`, `edge_mut(id)`
- `room(id)`, `wall(id)`, `opening(id)`
- `find_edge(point_a, point_b)`, `find_edge_mut(point_a, point_b)` — direction-agnostic
- `ensure_edge(point_a, point_b) -> Uuid`
- `ensure_contour_edges(points: &[Uuid])`
- `remove_point(id)` — cascade: delete edges referencing it, delete rooms/walls/openings referencing it
- `remove_room(id)`, `remove_wall(id)`, `remove_opening(id)`
- `remove_label(id)` — unchanged

Keep `Label` struct definition inline in project.rs or in a small label.rs.

### Key design notes

- `Label` is preserved (id, text, position, font_size, rotation) — it has no
  model dependencies.
- `distance_to_segment()` and `project_onto_segment()` free functions from the
  old `wall.rs` are useful for hit-testing. Move them to a utility module or
  into `edge.rs`.
- `point_in_polygon()` from old `room_metrics.rs` is useful for hit-testing.
  Move it to a geometry utility or into `room.rs`.

### Verification

```bash
cargo check -p smetana --lib 2>&1 | head -5
# Model layer should compile; expect errors in editor/ and app/
```

---

## Session 3 — Editor Layer Rewrite

**Goal:** Replace editor types, tools, and snap logic for the new model. After
this session editor/ compiles, but app/ still has broken references.

### Files to DELETE

| File | Reason |
|------|--------|
| `src/editor/room_detection.rs` | Rooms are manually created now |
| `src/editor/wall_joints.rs` | Walls are simple polygons, no joints |
| `src/editor/endpoint_merge.rs` | Points are shared by reference |
| `src/editor/wall_tool.rs` | Replaced by polygon-based tool |
| `src/editor/snap.rs` | Replaced by simplified snap |

### Files to CREATE

**`src/editor/snap.rs`** (new — simplified)
- `snap_to_point(world_pos, points, screen_radius, zoom) -> Option<Uuid>`
  — finds nearest existing point within screen pixel threshold
- `snap_to_grid(world_pos, grid_step) -> DVec2`
- `snap(world_pos, points, grid_step, zoom, snap_enabled) -> SnapResult`
  — tries point snap first, then grid snap

```rust
pub struct SnapResult {
    pub position: DVec2,
    pub snapped_point: Option<Uuid>,  // if snapped to existing point
}
```

### Files to MODIFY

**`src/editor/mod.rs`** (major rewrite)

```rust
pub enum Tool {
    Select,
    Point,
    Room,
    Wall,
    Door,
    Window,
}

pub enum Selection {
    None,
    Point(Uuid),
    Edge(Uuid),
    Opening(Uuid),
    Wall(Uuid),
    Room(Uuid),
}

/// State for the Room tool: collecting points for a contour.
pub struct RoomToolState {
    pub points: Vec<Uuid>,
    pub building_cutout: bool,
}

/// State for polygon-based tools (Wall, Door, Window): collecting points.
pub struct PolygonToolState {
    pub points: Vec<Uuid>,
}

pub struct EditorState {
    pub active_tool: Tool,
    pub selection: Selection,
    pub canvas: Canvas,
    pub room_tool: RoomToolState,
    pub polygon_tool: PolygonToolState,
    pub visibility: VisibilityMode,
}

pub enum VisibilityMode {
    All,
    Wireframe,  // points + edges only
    Rooms,      // points + rooms (no wall fills)
}
```

**`src/editor/canvas.rs`** — Minimal changes. Keep all viewport logic
(pan, zoom, world_to_screen, screen_to_world, draw_grid). The Canvas struct
and coordinate conversion are model-agnostic.

### Verification

```bash
# editor/ should compile in isolation
# app/ will still have errors (expected)
```

---

## Session 4 — Canvas Rendering

**Goal:** Rewrite all drawing code for the new model. Points and edges are
always drawn. Room/wall/opening fills respect VisibilityMode.

### Files to REWRITE

**`src/app/canvas_draw.rs`** (full rewrite, ~400–600 lines target)

Render order (back to front):
1. `draw_grid()` — already exists on Canvas, no change
2. `draw_room_fills()` — triangulated polygons via earcutr, with cutout holes;
   skip if VisibilityMode::Wireframe
3. `draw_wall_fills()` — simple filled polygons;
   skip unless VisibilityMode::All
4. `draw_opening_fills()` — filled polygons with door/window symbols;
   skip unless VisibilityMode::All
5. `draw_edges()` — lines between edge endpoint pairs
6. `draw_points()` — circles (6px radius), selected = blue, normal = gray
7. `draw_measurement_labels()` — edge distance text at midpoint, room name +
   area at centroid
8. `draw_tool_preview()` — ghost polygon for Room/Wall/Opening tool in progress

Key implementation details:
- Room fill: collect contour screen positions → `earcutr::earcut()` with
  hole indices for cutouts → draw triangles
- Wall fill: collect polygon screen positions → `earcutr::earcut()` →
  draw triangles with wall color
- Opening fill: similar to wall but with door arc / window cross-hatch symbol
- Edge drawing: simple line from point_a to point_b screen positions
- Point drawing: `painter.circle()` for each point
- Selection highlight: thicker stroke or glow for selected object
- Measurement labels: distance text at edge midpoint, rotated to edge angle

### Helper functions to implement

```rust
fn polygon_screen_coords(point_ids: &[Uuid], project: &Project, canvas: &Canvas, rect_center: Pos2) -> Vec<Pos2>
fn point_in_polygon(test: Pos2, polygon: &[Pos2]) -> bool
fn draw_door_symbol(painter, ...)   // arc symbol
fn draw_window_symbol(painter, ...) // cross-hatch symbol
```

### Verification

At this point, drawing code compiles but is not called yet (app/canvas.rs
still references old types). Will be connected in Session 5.

---

## Session 5 — Canvas Input Handling and Tools

**Goal:** Rewrite the canvas orchestrator and implement all tools. After this
session the canvas is fully functional — users can place points, create rooms,
draw walls, place openings, select/move/delete objects.

### Files to REWRITE

**`src/app/canvas.rs`** (full rewrite, ~500–700 lines target)

#### Orchestrator: `show_canvas()`

```
1. Allocate painter
2. Handle pan/zoom (keep existing logic)
3. Update cursor_world_pos
4. Dispatch to active tool handler
5. Call rendering functions from canvas_draw.rs
6. Draw status bar
```

No more room detection block (rooms are manual now).

#### Hit-testing: `hit_test()`

Priority order (front to back):
1. **Points** — circle hit-test, ~10px screen radius
2. **Edges** — distance to line segment, ~5px screen tolerance
3. **Openings** — point-in-polygon
4. **Walls** — point-in-polygon
5. **Rooms** — point-in-polygon (excluding cutouts)
6. **Nothing**

```rust
enum HitResult {
    Point(Uuid),
    Edge(Uuid),
    Opening(Uuid),
    Wall(Uuid),
    Room(Uuid),
    Nothing,
}
```

#### Tool implementations

**Select tool — `handle_select_tool()`**
- Click: run hit_test(), set selection
- Drag (when Point selected): move point position, all connected geometry
  updates automatically
- Delete key: cascade-delete selected object
- Escape: deselect

**Point tool — `handle_point_tool()`**
- Click: snap to existing point (select it) or snap to grid (create new
  Point with default height from ProjectDefaults)
- Snapshot before creation

**Room tool — `handle_room_tool()`**
- Click on existing point: add to room_tool.points
- If clicking the first point and len >= 3: finalize room
  - `history.snapshot()`, create Room, `ensure_contour_edges()`
  - Reset room_tool
- Enter key: finalize if >= 3 points
- Escape: cancel (clear room_tool)
- After room created, user can switch to cutout mode (UI button or hotkey)

**Wall tool — `handle_wall_tool()`**
- Click on existing point: add to polygon_tool.points
- Finalize: when clicking first point and len >= 3, or Enter key
  - Create Wall with default gray color
  - Reset polygon_tool
- Escape: cancel

**Door/Window tools — `handle_opening_tool()`**
- Click on existing point: add to polygon_tool.points
- Finalize: when clicking first point and len >= 3, or Enter key
  - Create Opening with default dimensions from ProjectDefaults
  - Set to Selection::Opening(id) for immediate property editing
  - Reset polygon_tool
- Escape: cancel

### Verification

```bash
cargo build   # Should compile (but properties panel may still have errors)
```

---

## Session 6 — UI Shell: App, Toolbar, Properties Panel

**Goal:** Wire up the App struct, toolbar, and properties panel for the new
model. After this session the full application compiles and runs.

### Files to MODIFY

**`src/app/mod.rs`** (significant rewrite)

Update `App` struct:
- Remove: `rooms_version` (no auto-detection)
- Keep: `screen`, `project`, `editor`, `history`, `edit_snapshot_version`,
  `status_message`, `last_saved_version`, `label_scale`
- Keep: project list fields (`project_entries`, `project_list_selection`, etc.)

Remove `merge_rooms()` entirely (no automatic room detection).

Update `delete_selected()` for new Selection variants:
```rust
match self.editor.selection {
    Selection::Point(id) => project.remove_point(id),
    Selection::Edge(id) => { /* remove edge by id */ },
    Selection::Room(id) => project.remove_room(id),
    Selection::Wall(id) => project.remove_wall(id),
    Selection::Opening(id) => project.remove_opening(id),
    Selection::Label(id) => project.remove_label(id),
    Selection::None => return,
}
```

Update `set_tool()` for new Tool enum — reset tool states on switch.

Update `eframe::App::update()`:
- Remove `show_price_list_window_ui()` and `show_service_picker_window()`
  (already gone from Session 1, but verify)

**`src/app/toolbar.rs`**

Update tool buttons:
```
Select (V) | Point (P) | Room (R) | Wall (W) | Door (D) | Window (O)
```

Update keyboard shortcuts accordingly (P for Point, R for Room).

Update `show_left_panel()`:
- Show counts: Points, Edges, Rooms, Walls, Openings, Labels
- Room list (clickable, sets selection)
- Label list (clickable, sets selection)
- Visibility mode toggle (All / Wireframe / Rooms)
- Label scale slider (keep)

Update `show_project_settings_window()`:
- Update defaults form for new ProjectDefaults (point_height instead of
  wall_thickness + wall_height)

Update `show_defaults_form()`:
- "Точка: Высота (мм)" instead of wall thickness/height
- Keep door and window defaults

**`src/app/properties_panel.rs`** (significant rewrite)

Replace all property editors with new ones:

*Point selected:*
- Position: X, Y (DragValue, mm)
- Height (DragValue, mm)
- "Used in" list: rooms, walls, openings referencing this point

*Edge selected:*
- Distance: DragValue (mm) with override/reset
- Computed distance (read-only label)
- Angle: DragValue (degrees) with override/reset (if applicable)
- Height at A, Height at B (read-only)
- Gross wall area (read-only, distance × avg height, in m²)

*Room selected:*
- Name (text edit)
- Perimeter (read-only, m)
- Floor area (read-only, m²)
- Point count, Cutout count
- [Add Cutout] button (switches room tool to cutout mode)
- [Delete Room] button

*Wall selected:*
- Color picker (RGBA)
- Point count

*Opening selected:*
- Kind (Door/Window — display label)
- Height, Width (DragValue)
- Sill height, Reveal width (Window only)
- Point count

*Label selected:*
- Text (text edit)
- Font size (DragValue)
- Rotation (DragValue)

**`src/app/property_edits.rs`**
- Keep `labeled_drag()` and `labeled_value()` helpers — they are model-agnostic
- Add `labeled_drag_override()` helper for edge distance/angle with
  override + reset button pattern

**`src/app/project_list.rs`**
- Update `show_new_project_dialog` defaults form for new ProjectDefaults
- Minimal changes otherwise

### Verification

```bash
cargo build    # Must compile cleanly
cargo clippy   # No warnings
cargo run      # App launches, all tools work, properties display correctly
```

---

## Session 7 — Persistence, Tests, and Final Cleanup

**Goal:** Update persistence for the new model format, write tests, clean up
dead code, update documentation. The app is fully functional after this session.

### Files to MODIFY

**`src/persistence.rs`**
- Update `load_project()`: remove the old wall side `ensure_sections()` fixup
  (no more sections). No special post-deserialization fixup needed for the new
  model.
- Rewrite `round_trip_project_with_wall` test → `round_trip_project`:
  - Create a project with points, edges, a room, a wall polygon, an opening
  - Save → load → verify all fields
- Keep project listing and deletion functions (unchanged)
- Remove price list functions if not already removed in Session 1

**New tests to write:**

*In model/edge.rs or model tests:*
- `test_edge_distance_computed` — default distance from point coords
- `test_edge_distance_override` — override takes precedence
- `test_shoelace_area_square` — 1000×1000mm square = 1,000,000 mm²
- `test_shoelace_area_triangle`

*In model/room.rs or model tests:*
- `test_room_perimeter` — 4-point room, verify sum of edge distances
- `test_room_floor_area` — coordinate-based area
- `test_room_floor_area_with_cutout` — area minus cutout

*In model/project.rs or model tests:*
- `test_ensure_edge_dedup` — calling ensure_edge twice returns same id
- `test_remove_point_cascades` — removing a point removes referencing
  edges, rooms, walls, openings

**`Cargo.toml`**
- Verify no unused dependencies remain (`rfd` still needed for file dialogs,
  `earcutr` still needed for triangulation, `image` check if still used)

**`CLAUDE.md`**
- Update Architecture section to describe new model
- Update Module Structure
- Update Key Design Decisions
- Remove references to: T-junctions, sections, SideData, wall joints,
  room detection, endpoint merge, wall_tool chaining
- Add references to: Point, Edge, Room (contour), Wall (visual polygon),
  Opening (polygon), VisibilityMode, cascade delete, ensure_edge

**Delete old saves:**
- Old project files in `saves/projects/` are incompatible with the new format.
  Note this in CLAUDE.md or a migration note. Users must create new projects.

### Verification

```bash
cargo build          # Clean
cargo test           # All tests pass
cargo clippy         # No warnings
cargo fmt -- --check # Formatted
cargo run            # Full functionality verified manually:
                     #   - Place points
                     #   - Create room from points
                     #   - Create wall polygon
                     #   - Place door/window
                     #   - Select/move/delete objects
                     #   - Edge distance override
                     #   - Room area/perimeter display
                     #   - Save/load project
                     #   - Undo/redo
```

---

## Session Summary

| # | Session | Compiles? | App Runs? | Key Output |
|---|---------|-----------|-----------|------------|
| 1 | Remove services, price list, export | Yes | Yes | Clean app, no service/export UI |
| 2 | New model types | Model only | No | Point, Edge, Room, Wall, Opening |
| 3 | Editor layer rewrite | Editor + model | No | New tools, snap, selection |
| 4 | Canvas rendering | Partial | No | New draw functions |
| 5 | Canvas input and tools | Partial | No | Hit-test, all tool handlers |
| 6 | UI shell (App, toolbar, properties) | Yes | Yes | Full UI wired up |
| 7 | Persistence, tests, cleanup | Yes | Yes | Tests, docs, final polish |

## Notes

- **Labels are preserved.** They are orthogonal to the model redesign (just
  text + position on canvas). The Label tool and label properties remain.
- **No data migration.** Old project files are incompatible. This is a
  major version bump.
- **Sessions 2–5 have a broken build.** This is intentional — the model change
  is foundational and cascades through the entire codebase. Each session fixes
  one layer, working bottom-up.
- **Utility functions to preserve:** `distance_to_segment()`,
  `project_onto_segment()`, and `point_in_polygon()` are needed for hit-testing.
  Move them from old modules to edge.rs or a geometry utility module in
  Session 2.
