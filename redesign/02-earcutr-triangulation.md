# Task 02: Replace hand-rolled triangulation with `earcutr`

**Phase:** 1 (drop-in replacement, no dependencies on other tasks)
**Estimated savings:** ~110 lines
**Depends on:** Nothing

## Goal

Replace the 117-line hand-rolled ear-clipping triangulation in `src/editor/triangulation.rs` with the `earcutr` crate (Rust port of Mapbox's earcut). Pure drop-in replacement with zero behavior change.

## Code Review

### Current implementation: `src/editor/triangulation.rs` (117 lines)

Five functions:
- `triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]>` — main entry, 80 lines
- `signed_area_from_indices()` — shoelace formula, 11 lines
- `cross()` — 2D cross product, 3 lines
- `point_in_triangle()` — barycentric test, 10 lines
- `sign()` — helper, 3 lines

Algorithm: O(n^2) average, O(n^3) worst case. No tests. The current code's degenerate-polygon handling is simplistic (allows one failed full pass before giving up).

**Call site:** Only called from `canvas_draw.rs:657`:
```rust
let triangles = crate::editor::triangulation::triangulate(&screen_pts);
```

Input is `Vec<egui::Pos2>` (screen-space room polygon, typically 4-12 vertices, f32).

### Why earcutr v0.5

- Version 0.5 introduced generic `Float` support — accepts `f32` directly, no conversion needed.
- Version 0.4 only accepted `f64`, requiring unnecessary `f32→f64` conversion.
- Minimal dependencies (`num-traits` only).
- Battle-tested algorithm (Mapbox earcut, millions of production users).
- Handles degenerate polygons better via z-order hashing.

### Alternatives considered

| Approach | Verdict |
|----------|---------|
| `earcutr` 0.5 (f32 native) | **Selected** — simplest API, f32 support, proven |
| `earcut` crate (ciscorn/earcut-rs) | Rejected — stateful API (`Earcut::new()`) is overkill for 4-12 vertex polygons |
| Simplify hand-rolled code | Rejected — still ~60 lines, still has edge-case bugs, still maintenance burden |
| `egui::Mesh` directly | Orthogonal — still needs a triangulation algorithm |

## Changes

### 1. Add dependency to `Cargo.toml`

```toml
earcutr = "0.5"
```

### 2. Replace `src/editor/triangulation.rs`

Replace the entire file (117 lines) with:

```rust
/// Triangulate a simple polygon using earcutr (earcut algorithm).
/// Input: vertices in order (CCW or CW).
/// Output: list of triangle index triples [i, j, k] referencing input vertices.
pub fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]> {
    if vertices.len() < 3 {
        return Vec::new();
    }
    let coords: Vec<f32> = vertices.iter().flat_map(|p| [p.x, p.y]).collect();
    let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
    indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()
}
```

Note: pass `f32` directly (v0.5 is generic over `Float`). No `as f64` conversion.

### 3. No changes to call site

The function signature is identical: `pub fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]>`.

## Verification

```bash
cargo build
cargo test
cargo run  # visual check: room fills should look identical
```
