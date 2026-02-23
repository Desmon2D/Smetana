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
│   ├── canvas_draw.rs       # Wall/opening/room/preview rendering (labels rotated along walls)
│   ├── toolbar.rs           # Top toolbar, left panel, keyboard shortcuts
│   ├── project_list.rs      # ProjectList startup screen
│   ├── properties_panel.rs  # Right panel: wall/opening/room property editors
│   ├── property_edits.rs    # Deferred property edit → history command flushing, validation
│   ├── price_list.rs        # Floating price list CRUD window
│   ├── service_picker.rs    # Service assignment picker dialog
│   └── services_panel.rs    # Assigned services display and quantity helpers
├── model/                   # Pure data types (serde-serializable)
│   ├── wall.rs              # Wall, Point2D, SideData, SectionData, SideJunction
│   ├── opening.rs           # Opening, OpeningKind (Door | Window)
│   ├── room.rs              # Room, WallSide
│   ├── project.rs           # Project, AssignedService, SideServices, WallSideServices
│   ├── price.rs             # PriceList, ServiceTemplate, UnitType, TargetObjectType
│   └── quantity.rs          # Quantity computation functions (wall/opening/room)
├── editor/                  # Canvas viewport and drawing tools
│   ├── canvas.rs            # Pan/zoom, world↔screen coordinate conversion, grid rendering
│   ├── wall_tool.rs         # Two-click wall creation state machine with chain support
│   ├── opening_tool.rs      # Door/window placement hover state
│   ├── snap.rs              # Snap: vertex (15px) > wall edge (T-junction) > grid > free (Shift)
│   ├── room_detection.rs    # Planar graph cycle detection for auto room detection
│   ├── room_metrics.rs      # Inner polygon, net/gross area, perimeter computation
│   ├── triangulation.rs     # Ear-clipping triangulation for room fill rendering
│   ├── wall_joints.rs       # Miter joint computation at wall junctions
│   └── wall_joints_miter.rs # Miter geometry helpers (2-wall and 3+ wall cases)
├── history.rs               # Command trait, all command structs, History (undo/redo stacks)
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
- **Coordinates in millimeters**: All model geometry (Point2D, wall dimensions, openings) uses mm. Canvas converts to screen pixels via zoom factor.
- **Wall sides have sections**: Each wall side (`SideData`) tracks T-junctions and auto-computes sections. `add_junction()` deduplicates by t position (within 0.001) to prevent zero-length sections.
- **OpeningKind enum**: Discriminated union (`Door { height, width }` | `Window { height, width, sill_height, reveal_width }`) — use pattern matching.
- **Room detection**: `WallGraph::build()` creates a planar graph from wall endpoints (merging within 5mm epsilon), force-merges T-junction endpoints with centerline vertices for connectivity, then `find_minimal_cycles()` uses minimum-angle traversal to detect rooms. The outer boundary (largest area) is excluded.
- **Wall tool chaining**: Two-click wall creation with chain support. `chain_start_snap` preserves the first click's snap across the entire chain so the closing wall can register a T-junction back at the chain origin. `start_junction_target` and `junction_target` on `AddWallCommand` handle T-junctions at both wall endpoints.
- **History**: Command pattern with `undo_stack` / `redo_stack`. Commands: `AddWallCommand`, `RemoveWallCommand`, `ModifyWallCommand`, `AddOpeningCommand`, `RemoveOpeningCommand`, `ModifyOpeningCommand`. The `version` counter increments on every push/undo/redo.
- **Deferred property edits**: DragValue mutations go directly to project fields. On selection change or before next command, `flush_property_edits()` compares against a snapshot and pushes a `ModifyWallCommand`/`ModifyOpeningCommand` if changed.
- **Services assigned per-object**: `Project.wall_services` is `HashMap<Uuid, WallSideServices>` (per-side, per-section). `opening_services` and `room_services` are `HashMap<Uuid, Vec<AssignedService>>`.
- **Canvas label scaling**: All canvas label font sizes are multiplied by `App.label_scale` (default 1.0, range 0.5–3.0). Controlled via a slider in the left panel. Affects wall thickness/section labels, room name/area labels, opening previews, and wall preview lengths.

### App Screens

`AppScreen` enum controls top-level navigation:
- `ProjectList` — startup screen listing saved projects
- `Editor` — main editor with toolbar, canvas, property panel, floating windows (price list, service picker)

### Quantity Computation

Service quantities (in `model/quantity.rs`) depend on `UnitType`:
- `Piece` → 1
- `SquareMeter` → net wall area (m²), reveal area for windows, floor area for rooms
- `LinearMeter` → wall length (m), door/window perimeter, room inner perimeter

### Persistence

- Projects: `saves/projects/{name}.json`
- Price lists: `saves/prices/{name}.json`
- Auto-save every frame when history version changes or dirty flag is set

## Conventions

- All dimensions are in millimeters internally; display converts to m/m² where needed
- Wall defaults: thickness 200mm, height 2700mm
- Door defaults: 2100×900mm
- Window defaults: 1400×1200mm, sill 900mm, reveal 250mm
- Wall area uses trapezoid formula: `length × (height_start + height_end) / 2`
- Window reveal perimeter: `2×height + 2×width` (all 4 sides)
- Door perimeter: `2×height + width` (no threshold)
