# Smetana

Desktop construction estimate application with a 2D floor plan editor. Built in Rust with [egui/eframe](https://github.com/emilk/egui).

Interface language: Russian. Target: low-end Windows hardware.

## Features

- **Point-first data model** — place points, then build rooms, walls, and openings as polygons referencing those points
- **Edge overrides** — override distances and angles for field measurements
- **Room area calculation** — Shoelace formula (coordinate-based or measurement-based with overrides)
- **Wall area** — gross and net (minus openings), displayed on edge labels
- **Door/window openings** — configurable dimensions, door swing direction, per-object colors
- **Cutouts** — subtract polygons from room area
- **Edge splitting** — click on an edge in Point tool to insert a new point, automatically updating all contours
- **Smart point removal** — deleting a point excises it from contours instead of cascade-deleting objects
- **Undo/redo** — snapshot-based history (100 entries)
- **Auto-save** — projects saved to `saves/projects/` as JSON
- **Pan/zoom** — mouse wheel zoom, middle-click or Space+drag to pan, WASD keys for camera movement
- **Snap** — points snap to existing points, edges, and grid; Shift disables snap

## Tools

| # | Tool | Description |
|---|------|-------------|
| 1 | Select | Click to select, drag to move points/labels, Delete to remove |
| 2 | Point | Place point (snaps to existing points, edges, grid) |
| 3 | Edge | Click two points to create an edge between them |
| 4 | Cutout | Click 3+ points to cut out from the containing room |
| 5 | Room | Click existing points to build a room contour |
| 6 | Door | Click existing points to build a door polygon |
| 7 | Window | Click existing points to build a window polygon |
| 8 | Wall | Click existing points to build a wall polygon |
| 9 | Label | Click to place a text annotation |

## Build & Run

Requires Rust edition 2024 (nightly or recent stable toolchain).

```bash
cargo build              # Build debug
cargo run                # Run the application
cargo test               # Run all tests
cargo clippy             # Lint
cargo fmt                # Format code
```

## License

All rights reserved.
