# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Detailed Documentation

See [`docs/CODEBASE_INDEX.md`](docs/CODEBASE_INDEX.md) for comprehensive codebase documentation: architecture diagrams, module map, all key types with fields, full public API surface, dependency graph, state management patterns, and build instructions. Start there to skip exploration and jump directly to implementation.
Read docs/CODEBASE_INDEX.md if you need to explore the code base.

## Project Overview

**Smetana** (Сметана) is a desktop construction estimate application built in Rust with egui/eframe. It provides a 2D floor plan editor where users draw walls, place doors/windows, auto-detect rooms, assign services from a price list, and generate Excel reports.

Interface language is Russian. Target: low-end Windows hardware.

## Build & Run Commands

```bash
cargo build              # Build debug
cargo run                # Run the application
cargo test               # Run all tests
cargo test round_trip    # Run a specific test by name
cargo clippy             # Lint
cargo fmt                # Format code
```

Rust edition: 2024. Requires a nightly or recent stable toolchain that supports edition 2024.

## Architecture

### Module Structure

```
src/
├── main.rs                  # Entry point, eframe initialization (1280x720 window)
├── app/                     # UI rendering and input handling
│   ├── mod.rs               # App struct, AppScreen enum, eframe::App impl, project management
│   ├── history.rs           # Snapshot-based History (undo/redo with VecDeque<Project>)
│   ├── canvas.rs            # Central panel: pan/zoom, tool dispatch, hit-testing, room detection
│   ├── canvas_draw.rs       # Wall/opening/room/preview rendering (two-pass: geometry then overlays)
│   ├── toolbar.rs           # Top toolbar, left panel, keyboard shortcuts, project settings window
│   ├── project_list.rs      # ProjectList startup screen
│   ├── properties_panel.rs  # Right panel: wall/opening/room property editors, SideInfo
│   ├── property_edits.rs    # Validation helpers, section editors, labeled_drag/labeled_value
│   ├── price_list.rs        # Floating price list CRUD window
│   ├── service_picker.rs    # Service assignment picker dialog
│   └── services_panel.rs    # Assigned services display
├── model/                   # Pure data types (serde-serializable)
│   ├── wall.rs              # Wall, SideData, SectionData, SideJunction; free fns distance_to_segment, project_onto_segment
│   ├── opening.rs           # Opening, OpeningKind (Door | Window), OpeningKind::target_type()
│   ├── room.rs              # Room, WallSide
│   ├── project.rs           # Project, ProjectDefaults, AssignedService, SideServices, WallSideServices, mutation/lookup methods
│   ├── price.rs             # PriceList, ServiceTemplate, UnitType, TargetObjectType
│   ├── quantity.rs          # Quantity computation functions (wall/opening/room/object)
│   └── room_metrics.rs      # Inner polygon, net/gross area, perimeter computation
├── editor/                  # Canvas viewport and drawing tools
│   ├── canvas.rs            # Pan/zoom, world↔screen coordinate conversion, grid rendering
│   ├── wall_tool.rs         # Two-click wall creation state machine with chain support
│   ├── snap.rs              # Snap: vertex (15px) > wall edge (T-junction) > grid > free (Shift)
│   ├── room_detection.rs    # Planar graph cycle detection for auto room detection
│   ├── wall_joints.rs       # Miter joint computation at wall junctions (world-space DVec2)
│   └── endpoint_merge.rs    # Shared endpoint merging utility for joints and room detection
├── export/
│   ├── excel.rs             # export_to_xlsx entry point, ExcelFormats struct, write helpers
│   └── excel_sheets.rs      # Per-sheet content writers (Rooms, Doors, Estimate)
└── persistence.rs           # Save/load project and price list JSON to saves/
```

### Key Design Decisions

- **app/ is split into focused files**: `App` struct and `eframe::App` impl live in `app/mod.rs`. UI rendering is split across `canvas.rs` (input), `canvas_draw.rs` (rendering), `toolbar.rs`, `properties_panel.rs`, `services_panel.rs`, etc. All methods are `pub(super)` on `App`.
- **Coordinates in millimeters**: All model geometry uses `glam::DVec2` for world-space coordinates (mm). Canvas converts to screen pixels via zoom factor. Free functions `distance_to_segment()` and `project_onto_segment()` in `model/wall.rs` replace the old `Point2D` methods.
- **Wall sides have sections**: Each wall side (`SideData`) tracks T-junctions and auto-computes sections. `add_junction()` deduplicates by t position (within 0.001) to prevent zero-length sections.
- **OpeningKind enum**: Discriminated union (`Door { height, width }` | `Window { height, width, sill_height, reveal_width }`) — use pattern matching. `target_type()` maps to `TargetObjectType`.
- **Room detection**: `WallGraph::build()` creates a planar graph from wall endpoints (using shared `merge_endpoints()` with 5mm epsilon), force-merges T-junction endpoints with centerline vertices for connectivity, then `find_minimal_cycles()` uses minimum-angle traversal to detect rooms. The outer boundary (largest area) is excluded. Detection is version-gated: only runs when `history.version != rooms_version`.
- **Wall tool chaining**: Two-click wall creation with chain support. `chain_start_snap` preserves the first click's snap across the entire chain so the closing wall can register a T-junction back at the chain origin. `Project::add_wall()` handles T-junction registration at both endpoints.
- **History (snapshot undo)**: `History` (in `app/history.rs`) stores `VecDeque<(Project, &'static str)>` for both undo and redo stacks. `snapshot()` clones the entire `Project` before mutation. `undo()`/`redo()` swap the whole project. 100-entry cap. `version` counter increments on every snapshot/undo/redo/mark_dirty.
- **Edit snapshot batching**: `edit_snapshot_version: Option<u64>` on `App` ensures DragValue property edits accumulate into a single undo step. One snapshot is taken when editing starts; reset on selection change or undo/redo.
- **Project mutation methods**: `Project::add_wall()`, `remove_wall()`, `add_opening()`, `remove_opening()`, `remove_label()`, `move_opening()` consolidate mutation logic (T-junction management, bidirectional wall↔opening links). Canvas calls `history.snapshot()` then these methods. Lookup methods `wall()`, `wall_mut()`, `opening()`, `opening_mut()`, `room()` provide convenient by-ID access.
- **Services assigned per-object**: `Project.wall_services` is `HashMap<Uuid, WallSideServices>` (per-side, per-section). `opening_services` and `room_services` are `HashMap<Uuid, Vec<AssignedService>>`.
- **Canvas label scaling**: All canvas label font sizes are multiplied by `App.label_scale` (default 1.0, range 0.5–3.0). Controlled via a slider in the left panel. Affects wall thickness/section labels, room name/area labels, opening previews, and wall preview lengths.
- **Per-project defaults**: `ProjectDefaults` (stored in `Project.defaults`, `#[serde(default)]` for backward compatibility) holds default dimensions for new walls (thickness, height), doors (height, width), and windows (height, width, sill, reveal). Configured at project creation and editable later via the "Настройки" floating window. `Wall::new()` takes explicit `thickness` and `height` parameters; opening creation constructs `OpeningKind` variants from project defaults.
- **Wall rendering two-pass**: `draw_walls()` splits into `draw_wall_geometry()` (pass 1: opaque section quads, junction ticks, wall outline) and `draw_wall_overlays()` (pass 2: selection highlights, endpoint circles, text labels). Hub polygons render between the two passes. `WallScreenGeometry` struct centralizes wall screen-space geometry (start/end, normals, half-thickness, lerp/left_at/right_at methods), replacing duplicated computation across draw functions. Door/window symbol rendering is extracted into `draw_door_symbol()` and `draw_window_symbol()` free functions. Each wall section is an opaque half-width polygon (centerline→edge) — no transparent overlays. Unselected walls use neutral gray fill; selected walls color each section with the shared `SECTION_COLORS` palette (global index across both sides — left sections first, then right — so every section gets a unique color). Section labels are always shown (colored when selected, neutral gray when not). Junction ticks only appear on selected walls.
- **Wall joints in world space**: `compute_joints()` operates entirely in world-space DVec2 (mm). `JointVertices` and `HubPolygon` store `DVec2` vertices. Screen-space conversion happens at render time in `canvas_draw.rs`. The shared `merge_endpoints()` utility (in `editor/endpoint_merge.rs`) consolidates endpoint merging logic used by both `wall_joints.rs` and `room_detection.rs`.
- **Canvas input decomposition**: `show_canvas()` delegates to `handle_pan_zoom()`, `draw_status_bar()`, `draw_empty_hint()`. `handle_select_tool()` dispatches to `handle_select_click()`, `handle_select_drag()`, `handle_select_keys()`. Hit-testing is extracted into free functions: `find_nearest_wall()`, `find_nearest_opening()`, `find_nearest_label()`.
- **UI helpers**: `labeled_drag()` and `labeled_value()` in `property_edits.rs` replace repetitive `ui.horizontal(|ui| { ui.label(...); ui.add(DragValue::new(...)); })` patterns across property editors.
- **Export helpers**: `ExcelFormats` struct consolidates all format definitions. `write_str()`, `write_num()`, `write_header_row()` replace repetitive `.write_*_with_format(...).map_err(...)` calls. `write_rooms_sheet` delegates to `write_rooms_summary()`, `write_room_walls_detail()`, `write_room_windows_detail()`.

### App Screens

`AppScreen` enum controls top-level navigation:
- `ProjectList` — startup screen listing saved projects
- `Editor` — main editor with toolbar, canvas, property panel, floating windows (price list, service picker, project settings)

### Quantity Computation

Service quantities (in `model/quantity.rs`) depend on `UnitType`:
- `Piece` → 1
- `SquareMeter` → net wall area (m²), reveal area for windows, floor area for rooms
- `LinearMeter` → wall length (m), door/window perimeter, room inner perimeter

### Persistence

- Projects: `saves/projects/{name}.json`
- Price lists: `saves/prices/{name}.json`
- Auto-save every frame when history version changes (compared via `last_saved_version`)

## Conventions

- All dimensions are in millimeters internally; display converts to m/m² where needed
- Wall defaults: thickness 200mm, height 2700mm (configurable per-project via `ProjectDefaults`)
- Door defaults: 2100×900mm (configurable per-project via `ProjectDefaults`)
- Window defaults: 1400×1200mm, sill 900mm, reveal 250mm (configurable per-project via `ProjectDefaults`)
- Wall area uses trapezoid formula: `length × (height_start + height_end) / 2`
- Window reveal perimeter: `2×height + 2×width` (all 4 sides)
- Door perimeter: `2×height + width` (no threshold)
