# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
├── main.rs              # Entry point, eframe initialization (1280x720 window)
├── app.rs               # Central App struct — ALL UI rendering and input handling
├── model/               # Pure data types (serde-serializable)
│   ├── wall.rs          # Wall, Point2D (coordinates in mm)
│   ├── opening.rs       # Opening, OpeningKind (Door | Window)
│   ├── room.rs          # Room, WallSide
│   ├── project.rs       # Project (walls, openings, rooms, assigned services)
│   └── price.rs         # PriceList, ServiceTemplate, UnitType, TargetObjectType
├── editor/              # Canvas viewport and drawing tools
│   ├── canvas.rs        # Pan/zoom, world↔screen coordinate conversion, grid rendering
│   ├── wall_tool.rs     # Two-click wall creation state machine (Idle → Drawing)
│   ├── opening_tool.rs  # Door/window placement hover state
│   ├── snap.rs          # Snap to vertex (15px radius) > grid > free (Shift held)
│   └── room_detection.rs # Planar graph cycle detection for auto room detection
├── history.rs           # Undo/redo via Command pattern (AddWall, RemoveWall, etc.)
├── export/excel.rs      # .xlsx report generation (3 sheets: Rooms, Doors, Estimate)
├── persistence/
│   ├── project_io.rs    # Save/load project JSON to saves/projects/
│   └── price_io.rs      # Save/load price list JSON to saves/prices/
└── panels/mod.rs        # Placeholder (panels are currently inline in app.rs)
```

### Key Design Decisions

- **app.rs is monolithic**: All UI layout, input handling, drawing, and business logic lives in `App::update()`. There are no separate panel modules yet — everything is rendered inline.
- **Coordinates in millimeters**: All model geometry (Point2D, wall dimensions, openings) uses mm. Canvas converts to screen pixels via zoom factor.
- **OpeningKind enum**: Discriminated union (`Door { height, width }` | `Window { height, width, sill_height, reveal_width }`) — use pattern matching.
- **Room detection**: `WallGraph::build()` creates a planar graph from wall endpoints (merging within 5mm epsilon), then `find_minimal_cycles()` uses minimum-angle traversal to detect rooms. The outer boundary (largest area) is excluded.
- **History**: Command pattern with `undo_stack` / `redo_stack`. Commands: `AddWallCommand`, `RemoveWallCommand`, `ModifyWallCommand`, `AddOpeningCommand`, `RemoveOpeningCommand`, `ModifyOpeningCommand`. The `version` counter increments on every push/undo/redo.
- **Services assigned per-object**: `Project.wall_services`, `opening_services`, `room_services` are `HashMap<Uuid, Vec<AssignedService>>`.

### App Screens

`AppScreen` enum controls top-level navigation:
- `ProjectList` — startup screen listing saved projects
- `Editor` — main editor with toolbar, canvas, property panel, bottom tabs (price list / assigned services)

### Quantity Computation

Service quantities depend on `UnitType`:
- `Piece` → 1
- `SquareMeter` → net wall area (m²), reveal area for windows, floor area for rooms
- `LinearMeter` → wall length (m), door/window perimeter, room inner perimeter

### Persistence

- Projects: `saves/projects/{name}.json`
- Price lists: `saves/prices/{name}.json`
- Auto-save on significant actions

## Conventions

- All dimensions are in millimeters internally; display converts to m/m² where needed
- Wall defaults: thickness 200mm, height 2700mm
- Door defaults: 2100×900mm
- Window defaults: 1400×1200mm, sill 900mm, reveal 250mm
- Wall area uses trapezoid formula: `length × (height_start + height_end) / 2`
- Window reveal perimeter: `2×height + 2×width` (all 4 sides)
- Door perimeter: `2×height + width` (no threshold)
