# Module Map

## `src/` — Root

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `main.rs` | 33 | Entry point. Configures eframe (1280x720, title "Сметана"), creates `App` | — |

## `src/model/` — Pure Data Types (serde-serializable)

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 16 | Re-exports all model types via `pub use` | all types and functions from submodules |
| `wall.rs` | 288 | `Wall`, `SideData`, `SectionData`, `SideJunction` — wall geometry, side dimensions, T-junction section management. Free functions `distance_to_segment()`, `project_onto_segment()` for DVec2 geometry | `Wall`, `SideData`, `SectionData`, `SideJunction`, `distance_to_segment`, `project_onto_segment` |
| `opening.rs` | 101 | `Opening`, `OpeningKind` — doors and windows with dimensions, wall attachment, and `target_type()` method | `Opening`, `OpeningKind` |
| `room.rs` | 46 | `Room`, `WallSide` — detected room with wall contour and interior side mapping | `Room`, `WallSide` |
| `label.rs` | 27 | `Label` — positioned text annotation with font size and rotation | `Label` |
| `project.rs` | 252 | `Project`, `ProjectDefaults`, `AssignedService`, `SideServices`, `WallSideServices` — top-level project container with mutation methods (`add_wall`, `remove_wall`, `add_opening`, `remove_opening`, `remove_label`, `move_opening`) and lookup methods (`wall`, `wall_mut`, `opening`, `opening_mut`, `room`) | `Project`, `ProjectDefaults`, `AssignedService`, `SideServices`, `WallSideServices` |
| `price.rs` | 96 | `PriceList`, `ServiceTemplate`, `UnitType`, `TargetObjectType` — price list and service definitions | `PriceList`, `ServiceTemplate`, `UnitType`, `TargetObjectType` |
| `quantity.rs` | 163 | Quantity computation functions for walls, openings, rooms, and generic objects | `opening_area_mm2`, `section_net_area`, `wall_side_quantity`, `wall_section_quantity`, `opening_quantity`, `room_quantity`, `compute_object_quantity` |
| `room_metrics.rs` | 200 | `RoomMetrics`, `compute_room_metrics()` — inner polygon, net/gross area, perimeter calculation | `RoomMetrics`, `compute_room_metrics` |

## `src/editor/` — Canvas Viewport and Drawing Tools

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 50 | `EditorTool`, `Selection`, `EditorState` enums/struct (incl. `OpeningTool`, `orphan_positions`). Re-exports tool types | `Canvas`, `WallGraph`, `SnapResult`, `SnapType`, `snap`, `WallTool`, `WallToolState`, `OpeningTool`, `EditorTool`, `Selection`, `EditorState` |
| `canvas.rs` | 178 | `Canvas` struct — pan/zoom viewport, world↔screen coordinate conversion (`screen_to_world_dvec2`), multi-level grid rendering | `Canvas` |
| `wall_tool.rs` | 59 | `WallTool`, `WallToolState` — two-click wall creation state machine with chain support | `WallTool`, `WallToolState` |
| `snap.rs` | 139 | `snap()` function, `SnapResult`, `SnapType` — cursor snapping (vertex > wall edge > grid > free). `SnapResult::wall_edge_junction()` helper | `snap`, `SnapResult`, `SnapType` |
| `room_detection.rs` | 400 | `WallGraph`, `GraphVertex`, `DirectedEdge` — planar graph cycle detection for automatic room detection. Uses `merge_endpoints()` | `WallGraph`, `GraphVertex`, `DirectedEdge` |
| `wall_joints.rs` | 440 | `compute_joints()`, `JointVertices`, `HubPolygon` — world-space miter joint computation at wall junctions. Uses `merge_endpoints()` | `compute_joints`, `JointVertices`, `HubPolygon` |
| `endpoint_merge.rs` | 30 | `merge_endpoints()` — shared utility for merging wall endpoints within epsilon distance | `merge_endpoints` |

## `src/app/` — UI Rendering and Input Handling

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 249 | `App` struct, `AppScreen` enum, `eframe::App` impl, project management, `delete_selected()`, `SECTION_COLORS` palette, `rooms_version` | `App` |
| `history.rs` | 61 | Snapshot-based `History`: `VecDeque<(Project, &'static str)>` undo/redo stacks, 100-entry cap, version counter | `History` |
| `canvas.rs` | 540 | `show_canvas()` — central panel orchestrator. Delegates to `handle_pan_zoom()`, `handle_select_click/drag/keys()`, hit-test functions (`find_nearest_wall/opening/label`), `draw_status_bar()`, `draw_empty_hint()`. Version-gated room detection | — (pub(super)) |
| `canvas_draw.rs` | 900 | `WallScreenGeometry` struct, `draw_wall_geometry()`/`draw_wall_overlays()` two-pass, `draw_attached_opening()`/`draw_orphaned_opening()`, `draw_door_symbol()`/`draw_window_symbol()`, `draw_rooms()`, `draw_wall_preview()`, `draw_opening_preview()` | — (pub(super)) |
| `toolbar.rs` | 318 | `show_toolbar()`, `show_left_panel()`, `handle_keyboard_shortcuts()`, `show_project_settings_window()` — top bar, left tree, hotkeys, settings | — (pub(super)) |
| `project_list.rs` | 142 | `show_project_list()` — startup screen with project CRUD | — (pub(super)) |
| `properties_panel.rs` | 310 | `show_right_panel()`, `SideInfo` struct with `compute()`/`empty()`, `show_side_panel()` — property editors for selected objects | — (pub(super)) |
| `property_edits.rs` | 130 | `has_validation_errors()`, `opening_errors()`, `selection_target_type()`, `show_side_sections()`, `labeled_drag()`, `labeled_value()` — validation, section editors, UI helpers | — (pub(super)) |
| `price_list.rs` | 183 | `show_price_list_window_ui()` — floating CRUD window for the price list | — (pub(super)) |
| `service_picker.rs` | 106 | `show_service_picker_window()` — dialog for assigning a service to an object | — (pub(super)) |
| `services_panel.rs` | 200 | Service display: `show_wall_side_services()`, `show_flat_services()`, `sync_custom_prices()` helper. Calls model quantity functions directly | — (pub(super)) |

## `src/persistence.rs` — Save/Load

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `persistence.rs` | 190 | `save_project()`, `load_project()`, `list_project_entries()`, `delete_project()`, `save_price_list()`, `load_price_list()`, `ProjectEntry` | `ProjectEntry`, `ensure_saves_dirs`, `project_path`, `save_project`, `load_project`, `list_projects`, `list_project_entries`, `delete_project`, `price_path`, `save_price_list`, `save_price_list_to`, `load_price_list` |

## `src/export/` — Excel Report Generation

| File | Lines | Purpose | Public Exports |
|------|------:|---------|----------------|
| `mod.rs` | 4 | Re-exports `excel` module | `export_to_xlsx` |
| `excel.rs` | 130 | `export_to_xlsx()`, `ExcelFormats` struct, `write_str()`/`write_num()`/`write_header_row()` helpers | `export_to_xlsx` |
| `excel_sheets.rs` | 410 | `write_rooms_sheet()` (delegates to `write_rooms_summary`, `write_room_walls_detail`, `write_room_windows_detail`), `write_doors_sheet()`, `write_estimate_sheet()` | `pub(super)` only |

## Totals

| Directory | Files | Lines |
|-----------|------:|------:|
| `src/model/` | 9 | ~1,090 |
| `src/editor/` | 7 | ~1,300 |
| `src/app/` | 11 | ~3,140 |
| `src/persistence.rs` | 1 | ~190 |
| `src/export/` | 3 | ~545 |
| `src/main.rs` | 1 | 33 |
| **Total** | **32** | **~6,300** |
