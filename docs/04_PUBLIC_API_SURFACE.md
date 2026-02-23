# Public API Surface

## `src/model/wall.rs` — Point2D, SideData, Wall

| Signature | Purpose |
|-----------|---------|
| `Point2D::new(x: f64, y: f64) -> Self` | Create point |
| `Point2D::distance_to(self, other: Point2D) -> f64` | Euclidean distance |
| `Point2D::distance_to_segment(self, a: Point2D, b: Point2D) -> f64` | Distance to line segment |
| `Point2D::project_onto_segment(self, a: Point2D, b: Point2D) -> (f64, Point2D)` | Project onto segment, returns (t, projected_point) |
| `SectionData::gross_area(&self) -> f64` | Section area in mm² (trapezoid formula) |
| `SideData::new(length: f64, height_start: f64, height_end: f64) -> Self` | Create with one implicit section |
| `SideData::gross_area(&self) -> f64` | Gross area mm² (trapezoid) |
| `SideData::section_count(&self) -> usize` | 1 if no junctions, N+1 if N junctions |
| `SideData::add_junction(&mut self, wall_id: Uuid, t: f64)` | Insert junction (sorted), recompute sections. Skips duplicates within 0.001 |
| `SideData::remove_junction(&mut self, wall_id: Uuid)` | Remove junction by wall ID, recompute |
| `SideData::ensure_sections(&mut self)` | Post-deserialization fixup (populates if empty) |
| `SideData::computed_total_length(&self, walls: &[Wall]) -> f64` | Section lengths + junction wall thicknesses |
| `SideData::recompute_sections(&mut self)` | Rebuild sections from junction t-values |
| `Wall::new(start: Point2D, end: Point2D) -> Self` | Create wall (thickness=200, height=2700) |
| `Wall::length(&self) -> f64` | Centerline length mm |
| `Wall::left_area(&self) -> f64` | Left side gross area mm² |
| `Wall::right_area(&self) -> f64` | Right side gross area mm² |

## `src/model/opening.rs` — OpeningKind, Opening

| Signature | Purpose |
|-----------|---------|
| `OpeningKind::default_door() -> Self` | Door 2100x900mm |
| `OpeningKind::default_window() -> Self` | Window 1400x1200mm, sill 900, reveal 250 |
| `OpeningKind::width(&self) -> f64` | Opening width mm |
| `OpeningKind::height(&self) -> f64` | Opening height mm |
| `Opening::new(kind: OpeningKind, wall_id: Option<Uuid>, offset_along_wall: f64) -> Self` | Create opening |
| `Opening::new_door(wall_id: Uuid, offset_along_wall: f64) -> Self` | Create default door |
| `Opening::new_window(wall_id: Uuid, offset_along_wall: f64) -> Self` | Create default window |

## `src/model/room.rs` — Room

| Signature | Purpose |
|-----------|---------|
| `Room::new(name: String, wall_ids: Vec<Uuid>, wall_sides: Vec<WallSide>) -> Self` | Create room |

## `src/model/project.rs` — Project, SideServices

| Signature | Purpose |
|-----------|---------|
| `Project::new(name: String) -> Self` | Create empty project |
| `SideServices::ensure_section(&mut self, section_index: usize) -> &mut Vec<AssignedService>` | Ensure section exists, return mutable ref |
| `SideServices::all_services(&self) -> impl Iterator<Item = &AssignedService>` | Flat iterator over all sections |
| `SideServices::is_empty(&self) -> bool` | True if no sections have services |

## `src/model/price.rs` — UnitType, TargetObjectType, ServiceTemplate, PriceList

| Signature | Purpose |
|-----------|---------|
| `UnitType::label(self) -> &'static str` | Russian label: "шт.", "м²", "п.м." |
| `TargetObjectType::label(self) -> &'static str` | Russian label: "Стена", "Окно", "Дверь", "Помещение" |
| `ServiceTemplate::new(name, unit_type, price_per_unit, target_type) -> Self` | Create template |
| `PriceList::new(name: String) -> Self` | Create empty price list |

**Constants:** `UnitType::ALL: [UnitType; 3]`, `TargetObjectType::ALL: [TargetObjectType; 4]`

## `src/model/quantity.rs` — Quantity Computation

| Signature | Purpose |
|-----------|---------|
| `opening_area_mm2(wall: &Wall, openings: &[Opening]) -> f64` | Total opening area on wall (mm²) |
| `wall_side_quantity(unit: UnitType, wall: &Wall, side: WallSide, openings: &[Opening]) -> f64` | Quantity for whole wall side |
| `wall_section_quantity(unit, wall, side, section_index, openings) -> f64` | Quantity for one section of a wall side |
| `opening_quantity(unit: UnitType, opening: &Opening) -> f64` | Quantity for an opening |
| `room_quantity(unit: UnitType, room: &Room, walls: &[Wall]) -> f64` | Quantity for a room |

## `src/editor/canvas.rs` — Canvas

| Signature | Purpose |
|-----------|---------|
| `Canvas::world_to_screen(&self, world: Pos2, rect_center: Pos2) -> Pos2` | World mm → screen px |
| `Canvas::screen_to_world(&self, screen: Pos2, rect_center: Pos2) -> Pos2` | Screen px → world mm |
| `Canvas::pan(&mut self, screen_delta: Vec2)` | Pan by screen-space delta |
| `Canvas::zoom_toward(&mut self, screen_pos: Pos2, rect_center: Pos2, factor: f32)` | Zoom keeping cursor point stable |
| `Canvas::draw_grid(&self, painter: &Painter, rect: Rect)` | Render 3-level grid + origin axes |

## `src/editor/snap.rs` — Snap

| Signature | Purpose |
|-----------|---------|
| `snap(world_pos: Point2D, grid_step: f64, zoom: f32, walls: &[Wall], shift_held: bool) -> SnapResult` | Compute snapped position. Priority: vertex > wall edge > grid > free |

**Constants:** `VERTEX_SNAP_SCREEN_PX: f64 = 15.0`

## `src/editor/wall_tool.rs` — WallTool

| Signature | Purpose |
|-----------|---------|
| `WallTool::reset(&mut self)` | Reset to Idle, clear all state |
| `WallTool::chain_from(&mut self, point: Point2D)` | Continue chain from endpoint |

## `src/editor/room_detection.rs` — WallGraph

| Signature | Purpose |
|-----------|---------|
| `WallGraph::build(walls: &[Wall]) -> Self` | Build planar graph from walls (merge endpoints, split at junctions, force-merge T-junction endpoints with centerline vertices to ensure connectivity) |
| `WallGraph::vertex_index_for_wall(&self, walls, wall_id, is_end) -> Option<usize>` | Find vertex index for a wall endpoint |
| `WallGraph::find_minimal_cycles(&self) -> Vec<Vec<DirectedEdge>>` | Find all minimal cycles (rooms), excluding outer boundary |
| `WallGraph::signed_area(&self, cycle: &[DirectedEdge]) -> f64` | Shoelace signed area of a cycle |
| `WallGraph::detect_rooms(&self, walls: &[Wall]) -> Vec<Room>` | Full pipeline: cycles → wall sides → Room structs |

**Constants:** `MERGE_EPSILON: f64 = 5.0`

## `src/editor/room_metrics.rs` — Room Metrics

| Signature | Purpose |
|-----------|---------|
| `compute_room_metrics(room: &Room, walls: &[Wall]) -> Option<RoomMetrics>` | Compute inner polygon, net/gross area, perimeter |

## `src/editor/triangulation.rs` — Triangulation

| Signature | Purpose |
|-----------|---------|
| `triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]>` | Ear-clipping triangulation for rendering |

## `src/editor/wall_joints.rs` — Wall Joint Rendering

| Signature | Purpose |
|-----------|---------|
| `compute_joints(walls: &[Wall], canvas: &Canvas, center: Pos2) -> (HashMap<(Uuid, bool), JointVertices>, Vec<HubPolygon>)` | Compute miter joints and hub polygons for all wall junctions |

**Constants:** `MERGE_EPS: f64 = 5.0`, `MAX_MITER_RATIO: f32 = 3.0`

## `src/history.rs` — Command System

### Command Trait

| Signature | Purpose |
|-----------|---------|
| `Command::execute(&mut self, project: &mut Project)` | Apply the command |
| `Command::undo(&mut self, project: &mut Project)` | Reverse the command |
| `Command::description(&self) -> &str` | Russian description string |

### History

| Signature | Purpose |
|-----------|---------|
| `History::new() -> Self` | Create empty history |
| `History::push(&mut self, cmd: Box<dyn Command>, project: &mut Project)` | Execute command, push to undo stack, clear redo |
| `History::push_already_applied(&mut self, cmd: Box<dyn Command>)` | Push without executing (for DragValue edits) |
| `History::undo(&mut self, project: &mut Project) -> bool` | Undo last command |
| `History::redo(&mut self, project: &mut Project) -> bool` | Redo last undone command |
| `History::can_undo(&self) -> bool` | Check if undo stack is non-empty |
| `History::can_redo(&self) -> bool` | Check if redo stack is non-empty |

### Command Variants

| Struct | Fields | execute() | undo() |
|--------|--------|-----------|--------|
| `AddWallCommand` | `wall: Wall`, `junction_target: Option<(Uuid, WallSide, f64)>`, `start_junction_target: Option<(Uuid, WallSide, f64)>` | Adds junction(s) to target wall side(s), pushes wall to `project.walls` | Removes junctions, removes wall |
| `RemoveWallCommand` | `wall: Wall`, `openings: Vec<Opening>`, `removed_junctions: Vec<(Uuid, WallSide, f64)>` | Detaches openings (sets fallback_position), removes junctions from other walls, removes wall | Restores wall, re-attaches openings, restores junctions |
| `ModifyWallCommand` | `wall_id: Uuid`, `old: WallProps`, `new: WallProps` | Sets wall thickness, left_side, right_side to `new` | Sets to `old` |
| `AddOpeningCommand` | `opening: Opening` | Links opening to wall's `openings` list, adds to `project.openings` | Unlinks from wall, removes from openings |
| `RemoveOpeningCommand` | `opening: Opening` | Unlinks from wall, removes from project | Restores opening and wall link |
| `ModifyOpeningCommand` | `opening_id: Uuid`, `old_kind: OpeningKind`, `new_kind: OpeningKind` | Sets opening kind to `new_kind` | Sets to `old_kind` |

**Constructor:** `RemoveWallCommand::new(wall_id, project) -> Option<Self>` — snapshots wall + attached openings + junctions referencing this wall.
**Constructor:** `RemoveOpeningCommand::new(opening_id, project) -> Option<Self>` — snapshots the opening.
**Constructor:** `ModifyOpeningCommand::from_values(opening_id, old_kind, new_kind) -> Self`

## `src/persistence/project_io.rs` — Project I/O

| Signature | Purpose |
|-----------|---------|
| `ensure_saves_dirs() -> Result<(), String>` | Create `saves/projects/` and `saves/prices/` if missing |
| `project_path(name: &str) -> PathBuf` | `saves/projects/{name}.json` |
| `save_project(project: &Project) -> Result<PathBuf, String>` | Serialize to JSON, write to `saves/projects/{name}.json` |
| `load_project(path: &Path) -> Result<Project, String>` | Read JSON, deserialize, run `ensure_sections()` fixup |
| `list_projects() -> Result<Vec<PathBuf>, String>` | List all `.json` files in saves directory |
| `list_project_entries() -> Result<Vec<ProjectEntry>, String>` | List projects with name, path, last-modified (sorted newest first) |
| `delete_project(path: &Path) -> Result<(), String>` | Delete project file |

## `src/persistence/price_io.rs` — Price List I/O

| Signature | Purpose |
|-----------|---------|
| `price_path(name: &str) -> PathBuf` | `saves/prices/{name}.json` |
| `save_price_list(price_list: &PriceList) -> Result<PathBuf, String>` | Save to default path |
| `save_price_list_to(price_list: &PriceList, path: &Path) -> Result<(), String>` | Save to arbitrary path (export) |
| `load_price_list(path: &Path) -> Result<PriceList, String>` | Load from path (import) |

## `src/export/excel.rs` — Excel Export

| Signature | Purpose |
|-----------|---------|
| `export_to_xlsx(project: &Project, price_list: &PriceList, path: &Path) -> Result<(), String>` | Generate 3-sheet .xlsx: "Помещения", "Двери", "Смета" |

## `src/app/mod.rs` — App

| Signature | Purpose |
|-----------|---------|
| `App::new(cc: &CreationContext) -> Self` | Initialize app with defaults, load project list |

### App Private Methods (pub(super))

| File | Method | Purpose |
|------|--------|---------|
| `canvas.rs` | `show_canvas(&mut self, ctx)` | Central panel: input handling, tool dispatch, room detection, rendering |
| `canvas_draw.rs` | `draw_walls(&self, painter, rect)` | Render all walls with joints, sections, labels |
| `canvas_draw.rs` | `draw_openings(&self, painter, rect)` | Render doors (arc) and windows (parallel lines) |
| `canvas_draw.rs` | `draw_rooms(&self, painter, rect)` | Render room fills (triangulated) with name/area labels |
| `canvas_draw.rs` | `draw_wall_preview(&self, painter, rect)` | Preview line for wall being drawn |
| `canvas_draw.rs` | `draw_opening_preview(&self, painter, rect)` | Preview rectangle for opening placement |
| `toolbar.rs` | `handle_keyboard_shortcuts(&mut self, ctx)` | Ctrl+Z/Y/S/N/O, V/W/D/O tool hotkeys |
| `toolbar.rs` | `show_toolbar(&mut self, ctx)` | Top panel: tool buttons, undo/redo, save, export, new project dialog |
| `toolbar.rs` | `show_left_panel(&mut self, ctx)` | Left panel: project structure tree, room list |
| `project_list.rs` | `show_project_list(&mut self, ctx)` | Startup screen: create/open/delete projects |
| `properties_panel.rs` | `show_right_panel(&mut self, ctx)` | Right panel: property editors, service lists |
| `property_edits.rs` | `update_edit_snapshots(&mut self)` | Detect selection change, flush if snapshot mismatch |
| `property_edits.rs` | `flush_property_edits(&mut self)` | Compare current props vs snapshot, push ModifyCommand if changed |
| `property_edits.rs` | `has_validation_errors(&self) -> bool` | Check for detached or out-of-bounds openings |
| `property_edits.rs` | `opening_errors(&self, opening) -> Vec<&str>` | List validation errors for an opening |
| `property_edits.rs` | `selection_target_type(&self) -> Option<TargetObjectType>` | Map current selection to target type |
| `property_edits.rs` | `show_side_sections(ui, side_data, side_id, section_net_areas, color_offset)` | Show per-section property editors (static method) |
| `price_list.rs` | `show_price_list_window_ui(&mut self, ctx)` | Floating window: add/edit/delete services, import/export |
| `service_picker.rs` | `show_service_picker_window(&mut self, ctx)` | Dialog for picking service to assign |
| `services_panel.rs` | `compute_wall_side_quantity(&self, unit, wall, side) -> f64` | Delegate to `model::wall_side_quantity` |
| `services_panel.rs` | `compute_wall_section_quantity(&self, unit, wall, side, idx) -> f64` | Delegate to `model::wall_section_quantity` |
| `services_panel.rs` | `compute_opening_quantity(&self, unit, opening) -> f64` | Delegate to `model::opening_quantity` |
| `services_panel.rs` | `compute_room_quantity(&self, unit, room) -> f64` | Delegate to `model::room_quantity` |
| `services_panel.rs` | `build_assigned_rows_for(&self, assigned, qty_fn) -> Vec<AssignedServiceRow>` | Build display rows for assigned services |
| `services_panel.rs` | `show_services_list(ui, grid_id, rows, prices) -> (Option<usize>, Option<usize>)` | Render service list, return (reset_idx, remove_idx) |
| `services_panel.rs` | `show_wall_side_services(&mut self, ui, wall_id, side, label, color_offset)` | Show services for wall side with per-section breakdown |
| `services_panel.rs` | `show_flat_services(&mut self, ui, obj_id, target, target_type, rows, services_map)` | Show services for opening/room |
