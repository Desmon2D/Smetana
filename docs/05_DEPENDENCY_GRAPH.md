# Dependency Graph

## Module-Level Import Map

### `src/main.rs`

| Depends On | Items |
|------------|-------|
| `crate::app` | `App` |
| `eframe` | `NativeOptions`, `egui::ViewportBuilder`, `run_native` |

### `src/app/mod.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::editor` | `EditorState`, `EditorTool` |
| `crate::history` | `History` |
| `crate::model` | `PriceList`, `Project`, `ProjectDefaults`, `Room`, `WallSide` |
| `crate::persistence` | `list_project_entries`, `load_project`, `save_project`, `ProjectEntry` |

### `src/app/canvas.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `glam` | `DVec2` |
| `crate::editor` | `EditorTool`, `Selection`, `SnapType`, `WallToolState`, `snap` |
| `crate::editor::room_detection` | `WallGraph` |
| `crate::model` | `Label`, `Opening`, `OpeningKind`, `Wall`, `distance_to_segment`, `project_onto_segment` |
| `super` | `App` |

### `src/app/canvas_draw.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::editor` | `EditorTool`, `Selection`, `SnapType`, `WallToolState` |
| `crate::editor::wall_joints` | `compute_joints` |
| `crate::model` | `OpeningKind` |
| `super` | `App`, `SECTION_COLORS` |

### `src/app/toolbar.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::editor` | `EditorTool`, `Selection` |
| `crate::export` | `export_to_xlsx` |
| `crate::model` | `ProjectDefaults` |
| `rfd` | `FileDialog` |
| `super` | `App`, `AppScreen` |

### `src/app/project_list.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::persistence` | `delete_project` |
| `super` | `App` |

### `src/app/properties_panel.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::editor` | `Selection` |
| `crate::editor::room_metrics` | `compute_room_metrics` |
| `crate::model` | `OpeningKind`, `TargetObjectType`, `WallSide`, `section_net_area` |
| `super` | `App`, `ServiceTarget` |

### `src/app/property_edits.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::editor` | `Selection` |
| `crate::model` | `Opening`, `OpeningKind`, `SideData`, `TargetObjectType` |
| `super` | `App`, `SECTION_COLORS` |

### `src/app/price_list.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::model` | `ServiceTemplate`, `TargetObjectType`, `UnitType` |
| `crate::persistence` | `load_price_list`, `save_price_list_to` |
| `rfd` | `FileDialog` |
| `super` | `App` |

### `src/app/service_picker.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::model` | `AssignedService`, `OpeningKind`, `TargetObjectType`, `WallSide` |
| `super` | `App`, `ServiceTarget` |

### `src/app/services_panel.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `crate::model` | `AssignedService`, `Project`, `TargetObjectType`, `UnitType`, `Wall`, `WallSide` |
| `super` | `App`, `SECTION_COLORS`, `ServiceTarget` |

### `src/history.rs`

| Depends On | Items |
|------------|-------|
| `std::collections` | `VecDeque` |
| `crate::model` | `Project` |

### `src/editor/mod.rs`

| Depends On | Items |
|------------|-------|
| `std::collections` | `HashMap` |
| `glam` | `DVec2` |
| `uuid` | `Uuid` |
| (re-exports) | `Canvas`, `OpeningTool`, `WallGraph`, `SnapResult`, `SnapType`, `snap`, `WallTool`, `WallToolState` |

### `src/editor/canvas.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |

### `src/editor/wall_tool.rs`

| Depends On | Items |
|------------|-------|
| `glam` | `DVec2` |
| `crate::editor::snap` | `SnapResult` |

### `src/editor/opening_tool.rs`

| Depends On | Items |
|------------|-------|
| `uuid` | `Uuid` |

### `src/editor/snap.rs`

| Depends On | Items |
|------------|-------|
| `glam` | `DVec2` |
| `uuid` | `Uuid` |
| `crate::model` | `Wall`, `WallSide`, `project_onto_segment` |

### `src/editor/room_detection.rs`

| Depends On | Items |
|------------|-------|
| `glam` | `DVec2` |
| `uuid` | `Uuid` |
| `std::collections` | `HashSet` |
| `crate::model` | `Room`, `Wall`, `WallSide` |

### `src/editor/room_metrics.rs`

| Depends On | Items |
|------------|-------|
| `glam` | `DVec2` |
| `crate::model` | `Room`, `Wall`, `WallSide`, `project_onto_segment` |

### `src/editor/triangulation.rs`

| Depends On | Items |
|------------|-------|
| `egui` | `Pos2` (implicit via crate) |
| `earcutr` | `earcut` |

### `src/editor/wall_joints.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `uuid` | `Uuid` |
| `std::collections` | `HashMap` |
| `crate::editor::canvas` | `Canvas` |
| `crate::model` | `Wall` |
| `super::wall_joints_miter` | `compute_two_wall_miter`, `compute_hub_polygon`, `line_line_intersection` |

### `src/editor/wall_joints_miter.rs`

| Depends On | Items |
|------------|-------|
| `eframe::egui` | UI framework |
| `uuid` | `Uuid` |
| `std::collections` | `HashMap` |
| `super::wall_joints` | `HubPolygon`, `JointVertices`, `WallAtJunction`, `MAX_MITER_RATIO` |

### `src/model/quantity.rs`

| Depends On | Items |
|------------|-------|
| `crate::editor::room_metrics` | `compute_room_metrics` |
| `crate::model` | `Opening`, `OpeningKind`, `Room`, `UnitType`, `Wall`, `WallSide` |

### `src/persistence/project_io.rs`

| Depends On | Items |
|------------|-------|
| `std::fs`, `std::path`, `std::time` | File I/O |
| `crate::model` | `Project` |

### `src/persistence/price_io.rs`

| Depends On | Items |
|------------|-------|
| `std::fs`, `std::path` | File I/O |
| `crate::model` | `PriceList` |
| `super::project_io` | `ensure_saves_dirs` |

### `src/export/excel.rs`

| Depends On | Items |
|------------|-------|
| `std::path` | `Path` |
| `rust_xlsxwriter` | `Format`, `FormatAlign`, `FormatBorder`, `Workbook` |
| `crate::model` | `PriceList`, `Project` |
| `super::excel_sheets` | `write_rooms_sheet`, `write_doors_sheet`, `write_estimate_sheet` |

### `src/export/excel_sheets.rs`

| Depends On | Items |
|------------|-------|
| `rust_xlsxwriter` | `Format`, `Worksheet` |
| `uuid` | `Uuid` |
| `chrono` | `Local` |
| `crate::editor::room_metrics` | `compute_room_metrics` |
| `crate::model` | `AssignedService`, `OpeningKind`, `PriceList`, `Project`, `UnitType`, `WallSide`, `opening_area_mm2`, `wall_side_quantity`, `opening_quantity`, `room_quantity` |

## Cross-Layer Summary

```
model/ ←── editor/ (snap, room_detection, room_metrics use model types + DVec2)
  ↑            ↑
  │            │
model/ ←── history.rs (snapshot clones entire Project)
  ↑            ↑
  │            │
model/ ←── app/ (all UI files read/write model through Project)
editor/ ←── app/ (canvas uses snap, room detection; drawing uses joints, metrics)
history ←── app/ (canvas + toolbar call snapshot/undo/redo)
  ↑
persistence ←── app/ (project_list, toolbar, price_list use save/load)
export ←── app/ (toolbar triggers xlsx export)

NOTE: model/quantity.rs depends on editor/room_metrics (cross-layer)
NOTE: export/excel_sheets.rs depends on editor/room_metrics (cross-layer)
```
