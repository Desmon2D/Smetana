# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Smetana** (Сметана) is a desktop construction estimate application built in Rust with egui/eframe. It provides a 2D floor plan editor with a **point-first data model**: users place points, then create rooms, walls, and openings as polygons referencing those points. Edge distances and angles can be overridden for field measurements.

Interface language is Russian. Target: low-end Windows hardware.

## Build & Run Commands

```bash
cargo build              # Build debug
cargo run                # Run the application
cargo test               # Run all tests (14 tests)
cargo clippy             # Lint
cargo fmt                # Format code
```

Rust edition: 2024. Requires a nightly or recent stable toolchain that supports edition 2024.

## Architecture

### Module Structure

```
src/
├── main.rs            # Entry point, eframe initialization (1280x720 window)
├── model.rs           # All data types (Point, Edge, Wall, Opening, Room, Label, Project,
│                      #   ProjectDefaults), geometry functions, tests (~820 lines)
├── persistence.rs     # Save/load project JSON to saves/projects/
└── app/               # UI rendering and input handling
    ├── mod.rs         # App struct (with editor fields inline), Tool, Selection, Canvas
    │                  #   viewport, Snap, History, ToolState, VisibilityMode (~565 lines)
    ├── canvas.rs      # Central panel: pan/zoom, tool dispatch, hit-testing, ALL rendering
    │                  #   (room/wall/opening fills, edges, points, labels, previews) (~1055 lines)
    ├── panels.rs      # Top toolbar, left panel, right properties panel, keyboard shortcuts,
    │                  #   project settings window, UI helpers (labeled_drag, etc.) (~750 lines)
    └── project_list.rs # ProjectList startup screen (~165 lines)
```

### Data Model (Point-First)

All geometry is built from **Points** as the fundamental primitive:

- **Point** — position (DVec2 in mm) + ceiling height. Shared by reference (Uuid) across all objects.
- **Edge** — connects two points. Distance/angle can be overridden for field measurements. Created automatically via `ensure_edge()` / `ensure_contour_edges()`.
- **Room** — ordered list of point UUIDs forming a closed contour, with optional cutouts. `floor_area()` uses Shoelace formula (coordinate-based by default, measurement-based when overrides exist).
- **Wall** — visual polygon (list of point UUIDs) with RGBA fill color.
- **Opening** — polygon (list of point UUIDs) with `OpeningKind` (Door or Window with dimensions).
- **Label** — free text annotation with position, font size, rotation.

### Key Design Decisions

- **Point-first architecture**: Points are the atomic primitives. Rooms, walls, and openings are ordered sets of point UUIDs. Edges connect point pairs and are auto-created when polygons are finalized. Deleting a point cascade-deletes all referencing objects.
- **Coordinates in millimeters**: All model geometry uses `glam::DVec2` for world-space coordinates (mm). Canvas converts to screen pixels via zoom factor.
- **Edge overrides**: Each edge has optional `distance_override` and `angle_override`. When set, room area computation switches from coordinate-based Shoelace to measurement-based polygon reconstruction.
- **OpeningKind enum**: `Door { height, width }` | `Window { height, width, sill_height, reveal_width }` — use pattern matching.
- **Manual room creation**: Users click existing points to define room contours. No automatic room detection. Cutouts are added via a button in the room properties panel.
- **Cascade delete**: `remove_point(id)` removes all edges, rooms, walls, and openings referencing that point. `remove_edge/room/wall/opening` only removes the specific object.
- **Edge deduplication**: `ensure_edge(a, b)` is direction-agnostic — returns existing edge whether stored as (a,b) or (b,a). `find_edge(a, b)` likewise.
- **History (snapshot undo)**: `History` stores `VecDeque<Project>` for undo/redo. `snapshot()` clones the entire `Project`. 100-entry cap. `version` counter increments on every mutation.
- **Edit snapshot batching**: `ensure_edit_snapshot()` checks `edit_snapshot_version` and takes a snapshot on first edit, so DragValue property edits accumulate into a single undo step.
- **Selection**: `Selection` enum has `.id() -> Option<Uuid>` to extract the ID regardless of variant. Type-specific checks use `==` comparison (e.g., `self.selection == Selection::Room(room.id)`).
- **resolve_positions**: `Project::resolve_positions(ids)` converts a slice of point UUIDs to `Vec<DVec2>`, used in hit-testing, polygon rendering, and area computation.
- **Canvas hit-testing**: Priority order (front to back): Points > Labels > Edges > Openings > Walls > Rooms. All hit-testing in world space with screen-pixel thresholds converted via zoom factor.
- **Contour tool pattern**: Room, Wall, Door, and Window tools share `handle_contour_tool()` and a single `ToolState { points, building_cutout }` — click existing points to collect UUIDs, close by clicking first point or pressing Enter, `finalize_contour()` creates the appropriate object.
- **Visibility modes**: `VisibilityMode::All` (everything), `Wireframe` (points + edges only), `Rooms` (points + rooms, no wall fills).
- **Canvas label scaling**: All canvas label font sizes multiplied by `App.label_scale` (default 1.0, range 0.5–3.0).
- **Per-project defaults**: `ProjectDefaults` holds default point height, door/window dimensions. Configured at project creation and editable via "Настройки" window.
- **Render order** (back to front): Grid → Room fills (earcutr triangulation with cutout holes) → Wall fills → Opening fills → Edges → Points → Measurement labels → Labels → Tool preview.
- **App struct**: Editor state (tool, selection, canvas, tool_state, visibility) lives directly in `App` — no separate `EditorState` wrapper. All `impl App` blocks in submodules access these fields directly.
- **UI helpers**: `labeled_drag()`, `labeled_value()`, `labeled_drag_override()` in `panels.rs` reduce boilerplate in property editors.

### App Screens

`AppScreen` enum controls top-level navigation:
- `ProjectList` — startup screen listing saved projects
- `Editor` — main editor with toolbar, canvas, property panel, project settings floating window

### Tools

| Tool | Key | Description |
|------|-----|-------------|
| Select | V | Click to select, drag to move points/labels, Delete to remove |
| Point | P | Click to place point (snap to existing or grid), Shift disables snap |
| Room | R | Click existing points to build contour, close on first point or Enter |
| Wall | W | Click existing points to build polygon, creates gray fill |
| Door | D | Click existing points to build polygon, creates Door opening |
| Window | O | Click existing points to build polygon, creates Window opening |
| Label | T | Click to place text label |

### Properties Panel

Selection-dependent editors:
- **Point**: X/Y position, height, "Used in" references list
- **Edge**: Distance override with reset, computed distance, angle override, heights, wall area
- **Room**: Name, floor area, perimeter, point/cutout counts, Add Cutout / Delete buttons
- **Wall**: Color picker, point count
- **Opening**: Kind label, dimensions (height/width, sill/reveal for windows), point count
- **Label**: Text, font size, rotation

### Persistence

- Projects: `saves/projects/{name}.json`
- Auto-save every frame when history version changes
- Old project files (pre-redesign) are incompatible — users must create new projects

## Conventions

- All dimensions are in millimeters internally; display converts to m/m² where needed
- Point height default: 2700mm (configurable per-project via `ProjectDefaults`)
- Door defaults: 2100×900mm (configurable per-project)
- Window defaults: 1400×1200mm, sill 900mm, reveal 250mm (configurable per-project)
- Wall area per edge: `distance × (height_a + height_b) / 2`
- Room area: Shoelace formula on point coordinates (or measurement-based reconstruction when overrides exist)
- Room perimeter: sum of edge distances around contour
