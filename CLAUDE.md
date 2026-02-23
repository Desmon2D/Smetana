# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Detailed Documentation

See [`docs/CODEBASE_INDEX.md`](docs/CODEBASE_INDEX.md) for comprehensive codebase documentation: architecture diagrams, module map, all key types with fields, full public API surface, dependency graph, state management patterns, and build instructions. Start there to skip exploration and jump directly to implementation.

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
│   ├── canvas.rs            # Central panel: pan/zoom input, tool dispatch, room detection trigger
│   ├── canvas_draw.rs       # Wall/opening/room/preview rendering (two-pass: geometry then overlays)
│   ├── toolbar.rs           # Top toolbar, left panel, keyboard shortcuts, project settings window
│   ├── project_list.rs      # ProjectList startup screen
│   ├── properties_panel.rs  # Right panel: wall/opening/room property editors
│   ├── property_edits.rs    # Validation helpers, section property editors
│   ├── price_list.rs        # Floating price list CRUD window
│   ├── service_picker.rs    # Service assignment picker dialog
│   └── services_panel.rs    # Assigned services display and quantity helpers
├── model/                   # Pure data types (serde-serializable)
│   ├── wall.rs              # Wall, SideData, SectionData, SideJunction; free fns distance_to_segment, project_onto_segment
│   ├── opening.rs           # Opening, OpeningKind (Door | Window)
│   ├── room.rs              # Room, WallSide
│   ├── project.rs           # Project, ProjectDefaults, AssignedService, SideServices, WallSideServices, mutation methods
│   ├── price.rs             # PriceList, ServiceTemplate, UnitType, TargetObjectType
│   └── quantity.rs          # Quantity computation functions (wall/opening/room)
├── editor/                  # Canvas viewport and drawing tools
│   ├── canvas.rs            # Pan/zoom, world↔screen coordinate conversion, grid rendering
│   ├── wall_tool.rs         # Two-click wall creation state machine with chain support
│   ├── opening_tool.rs      # Door/window placement hover state
│   ├── snap.rs              # Snap: vertex (15px) > wall edge (T-junction) > grid > free (Shift)
│   ├── room_detection.rs    # Planar graph cycle detection for auto room detection
│   ├── room_metrics.rs      # Inner polygon, net/gross area, perimeter computation
│   ├── triangulation.rs     # earcutr-based triangulation for room fill rendering
│   ├── wall_joints.rs       # Miter joint computation at wall junctions
│   └── wall_joints_miter.rs # Miter geometry helpers (2-wall and 3+ wall cases)
├── history.rs               # Snapshot-based History (undo/redo with VecDeque<Project>)
├── export/
│   ├── excel.rs             # export_to_xlsx entry point (3-sheet workbook)
│   └── excel_sheets.rs      # Per-sheet content writers (Rooms, Doors, Estimate)
├── persistence/
│   ├── project_io.rs        # Save/load project JSON to saves/projects/
│   └── price_io.rs          # Save/load price list JSON to saves/prices/
└── panels/mod.rs            # Placeholder (panels are inline in app/)
```

### Key Design Decisions

- **app/ is split into focused files**: `App` struct and `eframe::App` impl live in `app/mod.rs`. UI rendering is split across `canvas.rs` (input), `canvas_draw.rs` (rendering), `toolbar.rs`, `properties_panel.rs`, `services_panel.rs`, etc. All methods are `pub(super)` on `App`.
- **Coordinates in millimeters**: All model geometry uses `glam::DVec2` for world-space coordinates (mm). Canvas converts to screen pixels via zoom factor. Free functions `distance_to_segment()` and `project_onto_segment()` in `model/wall.rs` replace the old `Point2D` methods.
- **Wall sides have sections**: Each wall side (`SideData`) tracks T-junctions and auto-computes sections. `add_junction()` deduplicates by t position (within 0.001) to prevent zero-length sections.
- **OpeningKind enum**: Discriminated union (`Door { height, width }` | `Window { height, width, sill_height, reveal_width }`) — use pattern matching.
- **Room detection**: `WallGraph::build()` creates a planar graph from wall endpoints (merging within 5mm epsilon), force-merges T-junction endpoints with centerline vertices for connectivity, then `find_minimal_cycles()` uses minimum-angle traversal to detect rooms. The outer boundary (largest area) is excluded.
- **Wall tool chaining**: Two-click wall creation with chain support. `chain_start_snap` preserves the first click's snap across the entire chain so the closing wall can register a T-junction back at the chain origin. `Project::add_wall()` handles T-junction registration at both endpoints.
- **History (snapshot undo)**: `History` stores `VecDeque<(Project, &'static str)>` for both undo and redo stacks. `snapshot()` clones the entire `Project` before mutation. `undo()`/`redo()` swap the whole project. 100-entry cap. `version` counter increments on every snapshot/undo/redo/mark_dirty.
- **Edit snapshot batching**: `edit_snapshot_version: Option<u64>` on `App` ensures DragValue property edits accumulate into a single undo step. One snapshot is taken when editing starts; reset on selection change or undo/redo.
- **Project mutation methods**: `Project::add_wall()`, `remove_wall()`, `add_opening()`, `remove_opening()`, `remove_label()` consolidate mutation logic (T-junction management, bidirectional wall↔opening links). Canvas calls `history.snapshot()` then these methods.
- **Services assigned per-object**: `Project.wall_services` is `HashMap<Uuid, WallSideServices>` (per-side, per-section). `opening_services` and `room_services` are `HashMap<Uuid, Vec<AssignedService>>`.
- **Canvas label scaling**: All canvas label font sizes are multiplied by `App.label_scale` (default 1.0, range 0.5–3.0). Controlled via a slider in the left panel. Affects wall thickness/section labels, room name/area labels, opening previews, and wall preview lengths.
- **Per-project defaults**: `ProjectDefaults` (stored in `Project.defaults`, `#[serde(default)]` for backward compatibility) holds default dimensions for new walls (thickness, height), doors (height, width), and windows (height, width, sill, reveal). Configured at project creation and editable later via the "Настройки" floating window. `Wall::new()` takes explicit `thickness` and `height` parameters; opening creation constructs `OpeningKind` variants from project defaults.
- **Wall rendering two-pass**: `draw_walls()` uses a two-pass approach. Pass 1 draws geometry (opaque section quads per side, junction ticks, wall outline, hub polygons). Pass 2 draws overlays on top (selection highlights, endpoint circles, text labels). This ensures indicators and labels are never hidden by joint fills. Each wall section is an opaque half-width polygon (centerline→edge) — no transparent overlays. Unselected walls use neutral gray fill; selected walls color each section with the shared `SECTION_COLORS` palette (global index across both sides — left sections first, then right — so every section gets a unique color). Section labels are always shown (colored when selected, neutral gray when not). Junction ticks only appear on selected walls.

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
