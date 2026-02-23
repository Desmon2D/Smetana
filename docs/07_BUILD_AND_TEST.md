# Build and Test

## Commands

| Command | Purpose |
|---------|---------|
| `cargo build` | Build debug binary |
| `cargo run` | Run the application |
| `cargo test` | Run all tests |
| `cargo test round_trip` | Run a specific test by name substring |
| `cargo clippy` | Lint with Clippy |
| `cargo fmt` | Format code with rustfmt |

## Toolchain Requirements

| Requirement | Value |
|-------------|-------|
| Rust edition | **2024** |
| Toolchain | Nightly or recent stable that supports edition 2024 |
| Target platform | **Windows** (primary: low-end Windows hardware) |
| Window size | 1280 x 720 pixels |

## Dependencies — `Cargo.toml`

| Crate | Version | Purpose |
|-------|---------|---------|
| `eframe` | 0.31 | Desktop application framework (wraps egui with native windowing via winit + OpenGL) |
| `egui` | 0.31 | Immediate-mode GUI library — all UI rendering and input handling |
| `serde` | 1 (features: `derive`) | Serialization/deserialization framework for all model types |
| `serde_json` | 1 | JSON serialization for project and price list persistence |
| `uuid` | 1 (features: `v4`, `serde`) | Unique identifiers for all model objects (walls, openings, rooms, services) |
| `rfd` | 0.15 | Native file dialogs (open/save for projects, price lists, Excel export) |
| `rust_xlsxwriter` | 0.93 | Excel .xlsx file generation (3-sheet report: rooms, doors, estimate) |

## Platform Notes

- **Windows-only target**: Uses `eframe` with native WGL/EGL backend via `glutin`. No cross-platform CI configured.
- **File paths**: Project saves go to `saves/projects/{name}.json` and price lists to `saves/prices/{name}.json` relative to the working directory.
- **Auto-save**: Saves on every frame where the history version has changed or `dirty` flag is set. No debouncing.
- **No external runtime dependencies**: Everything is statically linked via Cargo.

## Tests

| Test | File | Description |
|------|------|-------------|
| `round_trip_project_with_wall` | `src/persistence/project_io.rs:103` | Saves a project with one wall, loads it back, verifies all fields match |
| `round_trip_price_list` | `src/persistence/price_io.rs:47` | Saves a price list with two services, loads it back, verifies fields |

Both tests create temporary files in `saves/` and clean up after themselves.

## Build Output

- Debug binary: `target/debug/smetana.exe`
- Release binary: `target/release/smetana.exe` (use `cargo build --release`)
