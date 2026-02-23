# Module Map

## `src/` — Root

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `main.rs` | 35 | Entry point. Configures eframe (1280x720, title "Сметана"), creates `App` | — |

## `src/model/` — Pure Data Types (serde-serializable)

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 15 | Re-exports all model types via `pub use` | all types and functions from submodules |
| `wall.rs` | 288 | `Wall`, `SideData`, `SectionData`, `SideJunction` — wall geometry, side dimensions, T-junction section management. Free functions `distance_to_segment()`, `project_onto_segment()` for DVec2 geometry | `Wall`, `SideData`, `SectionData`, `SideJunction`, `distance_to_segment`, `project_onto_segment` |
| `opening.rs` | 93 | `Opening`, `OpeningKind` — doors and windows with dimensions and wall attachment | `Opening`, `OpeningKind` |
| `room.rs` | 46 | `Room`, `WallSide` — detected room with wall contour and interior side mapping | `Room`, `WallSide` |
| `label.rs` | 27 | `Label` — positioned text annotation with font size and rotation | `Label` |
| `project.rs` | 185 | `Project`, `ProjectDefaults`, `AssignedService`, `SideServices`, `WallSideServices` — top-level project container with service assignments and mutation methods (`add_wall`, `remove_wall`, `add_opening`, `remove_opening`, `remove_label`) | `Project`, `ProjectDefaults`, `AssignedService`, `SideServices`, `WallSideServices` |
| `price.rs` | 96 | `PriceList`, `ServiceTemplate`, `UnitType`, `TargetObjectType` — price list and service definitions | `PriceList`, `ServiceTemplate`, `UnitType`, `TargetObjectType` |
| `quantity.rs` | 141 | Quantity computation functions for walls, openings, and rooms | `opening_area_mm2`, `section_net_area`, `wall_side_quantity`, `wall_section_quantity`, `opening_quantity`, `room_quantity` |

## `src/editor/` — Canvas Viewport and Drawing Tools

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 63 | `EditorTool`, `Selection`, `EditorState` enums/struct (incl. `orphan_positions` for detached openings). Re-exports tool types | `Canvas`, `OpeningTool`, `WallGraph`, `SnapResult`, `SnapType`, `snap`, `WallTool`, `WallToolState`, `EditorTool`, `Selection`, `EditorState` |
| `canvas.rs` | 172 | `Canvas` struct — pan/zoom viewport, world↔screen coordinate conversion, multi-level grid rendering | `Canvas` |
| `wall_tool.rs` | 59 | `WallTool`, `WallToolState` — two-click wall creation state machine with chain support | `WallTool`, `WallToolState` |
| `opening_tool.rs` | 21 | `OpeningTool` — hover state for door/window placement preview | `OpeningTool` |
| `snap.rs` | 130 | `snap()` function, `SnapResult`, `SnapType` — cursor snapping (vertex > wall edge > grid > free) | `snap`, `SnapResult`, `SnapType` |
| `room_detection.rs` | 389 | `WallGraph`, `GraphVertex`, `DirectedEdge` — planar graph cycle detection for automatic room detection | `WallGraph`, `GraphVertex`, `DirectedEdge` |
| `room_metrics.rs` | 280 | `RoomMetrics`, `compute_room_metrics()` — inner polygon, net/gross area, perimeter calculation | `RoomMetrics`, `compute_room_metrics` |
| `triangulation.rs` | 11 | `triangulate()` — earcutr-based triangulation for room fill rendering | `triangulate` |
| `wall_joints.rs` | 313 | `compute_joints()`, `JointVertices`, `HubPolygon` — miter joint computation at wall junctions | `compute_joints`, `JointVertices`, `HubPolygon` |
| `wall_joints_miter.rs` | 171 | `compute_two_wall_miter()`, `compute_hub_polygon()`, `line_line_intersection()` — miter geometry helpers | `pub(super)` only |

## `src/app/` — UI Rendering and Input Handling

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 242 | `App` struct, `AppScreen` enum, `eframe::App` impl, project management, `delete_selected()`, `SECTION_COLORS` palette | `App` |
| `canvas.rs` | 525 | `show_canvas()` — central panel: pan/zoom input, tool dispatch (wall/select/opening/label), room detection trigger | — (pub(super)) |
| `canvas_draw.rs` | 815 | `draw_walls()` (two-pass: opaque section quads + overlays), `draw_openings()`, `draw_rooms()`, `draw_wall_preview()`, `draw_opening_preview()`, plus `paint_rotated_text()` helper for wall-aligned label rendering | — (pub(super)) |
| `toolbar.rs` | 318 | `show_toolbar()`, `show_left_panel()`, `handle_keyboard_shortcuts()`, `show_project_settings_window()` — top bar, left tree, hotkeys, settings | — (pub(super)) |
| `project_list.rs` | 142 | `show_project_list()` — startup screen with project CRUD | — (pub(super)) |
| `properties_panel.rs` | 450 | `show_right_panel()` — property editors for selected wall/opening/room/label + service lists | — (pub(super)) |
| `property_edits.rs` | 130 | `has_validation_errors()`, `opening_errors()`, `selection_target_type()`, `show_side_sections()` — validation and section editors | — (pub(super)) |
| `price_list.rs` | 183 | `show_price_list_window_ui()` — floating CRUD window for the price list | — (pub(super)) |
| `service_picker.rs` | 106 | `show_service_picker_window()` — dialog for assigning a service to an object | — (pub(super)) |
| `services_panel.rs` | 268 | Service display helpers: `show_wall_side_services()`, `show_flat_services()`, quantity wrappers | — (pub(super)) |

## `src/history.rs` — Undo/Redo

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `history.rs` | 61 | Snapshot-based `History`: `VecDeque<(Project, &'static str)>` undo/redo stacks, 100-entry cap, version counter | `History` |

## `src/persistence/` — Save/Load

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 5 | Re-exports from `project_io` and `price_io` | all pub items from submodules |
| `project_io.rs` | 131 | `save_project()`, `load_project()`, `list_project_entries()`, `delete_project()`, `ProjectEntry` | `ProjectEntry`, `ensure_saves_dirs`, `project_path`, `save_project`, `load_project`, `list_projects`, `list_project_entries`, `delete_project` |
| `price_io.rs` | 74 | `save_price_list()`, `load_price_list()`, `save_price_list_to()` | `price_path`, `save_price_list`, `save_price_list_to`, `load_price_list` |

## `src/export/` — Excel Report Generation

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 4 | Re-exports `excel` module | `export_to_xlsx` |
| `excel.rs` | 64 | `export_to_xlsx()` — creates 3-sheet workbook, delegates to sheet writers | `export_to_xlsx` |
| `excel_sheets.rs` | 557 | `write_rooms_sheet()`, `write_doors_sheet()`, `write_estimate_sheet()` — per-sheet content | `pub(super)` only |

## `src/panels/` — Placeholder

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 1 | Empty placeholder (panels are inline in `app/`) | — |

## Totals

| Directory | Files | Lines |
|-----------|------:|------:|
| `src/model/` | 8 | 891 |
| `src/editor/` | 10 | 1,609 |
| `src/app/` | 10 | 3,179 |
| `src/history.rs` | 1 | 61 |
| `src/persistence/` | 3 | 210 |
| `src/export/` | 3 | 625 |
| `src/main.rs` | 1 | 35 |
| `src/panels/` | 1 | 1 |
| **Total** | **37** | **~6,600** |
