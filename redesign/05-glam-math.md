# Task 05: Replace Point2D with glam::DVec2

**Phase:** 1 (drop-in replacement, no dependencies on other tasks)
**Estimated savings:** ~20-30 lines (real value is arithmetic readability)
**Depends on:** Nothing

## Goal

Replace the 37-line hand-rolled `Point2D` struct with `glam::DVec2`. The primary benefit is not line count but arithmetic readability — `a + d * t` instead of `Point2D::new(a.x + d_x * t, a.y + d_y * t)`. Eliminates manual component-wise math in room_metrics, room_detection, and snap modules.

## Code Review

### Current `Point2D` definition (`src/model/wall.rs:28-65`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D { pub x: f64, pub y: f64 }

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self                        // 3 lines
    pub fn distance_to(self, other: Point2D) -> f64            // 3 lines
    pub fn distance_to_segment(self, a, b) -> f64              // 3 lines
    pub fn project_onto_segment(self, a, b) -> (f64, Point2D)  // 13 lines
}
```

### Usage: 12 files, ~65 references

Key files: `wall.rs`, `label.rs`, `opening.rs`, `room.rs` (model, serialized), `snap.rs`, `room_detection.rs`, `room_metrics.rs` (editor), `canvas.rs`, `canvas_draw.rs` (app).

### Method mapping

| Point2D | glam::DVec2 |
|---------|-------------|
| `Point2D::new(x, y)` | `DVec2::new(x, y)` |
| `a.distance_to(b)` | `a.distance(b)` |
| `a.distance_to_segment(a, b)` | `distance_to_segment(a, b, c)` (free fn) |
| `a.project_onto_segment(a, b)` | `project_onto_segment(a, b, c)` (free fn) |

### Serde compatibility: VERIFIED

`DVec2` with `features = ["serde"]` serializes as `{"x": ..., "y": ...}` — identical to current `Point2D` JSON format. Existing saved projects load without migration.

### Arithmetic simplification examples

**`room_metrics.rs` line_intersection (7 lines → 4 lines):**
```rust
// Before:                                    // After:
let d1x = a2.x - a1.x;                       let d1 = a2 - a1;
let d1y = a2.y - a1.y;                       let d2 = b2 - b1;
let d2x = b2.x - b1.x;                       let denom = d1.perp_dot(d2);
let d2y = b2.y - b1.y;                       let t = (b1 - a1).perp_dot(d2) / denom;
let denom = d1x * d2y - d1y * d2x;           Some(a1 + d1 * t)
let t = ((b1.x-a1.x)*d2y-(b1.y-a1.y)*d2x)/denom;
Some(Point2D::new(a1.x + t*d1x, a1.y + t*d1y))
```

**`snap.rs` edge offset (2 lines → 1 line):**
```rust
// Before:
let edge_start = Point2D::new(wall.start.x + lnx * sign, wall.start.y + lny * sign);
// After:
let edge_start = wall.start + normal * sign;
```

### Alternatives considered

| Approach | Verdict |
|----------|---------|
| Full `DVec2` rename (no alias) | **Selected** — clean, unambiguous, ~65 references is manageable |
| `type Point2D = DVec2` alias | Rejected — creates indefinite two-name confusion |
| Add operator overloads to Point2D | Rejected — +39 lines added for manual math, maintenance burden |
| Newtype wrapper `Point2D(DVec2)` | Rejected — no second vector type to confuse with, `Deref` quirks with operators |

## Changes

Execute in two commits for safe review.

### Commit 1: Mechanical replacement (no behavior change)

#### 1a. Add dependency to `Cargo.toml`

```toml
glam = { version = "0.29", features = ["serde"] }
```

#### 1b. Replace `Point2D` in `src/model/wall.rs`

Delete the `Point2D` struct (lines 28-34) and its `impl` block (lines 36-65). Replace with:

```rust
use glam::DVec2;

/// Distance from point `p` to the line segment from `a` to `b`.
pub fn distance_to_segment(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let (_, proj) = project_onto_segment(p, a, b);
    p.distance(proj)
}

/// Project point `p` onto the line segment from `a` to `b`.
/// Returns (t, projected_point) where t is in [0, 1].
pub fn project_onto_segment(p: DVec2, a: DVec2, b: DVec2) -> (f64, DVec2) {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-12 {
        return (0.0, a);
    }
    let t = (p - a).dot(ab) / len_sq;
    let t = t.clamp(0.0, 1.0);
    (t, a + ab * t)
}
```

#### 1c. Update `src/model/mod.rs` re-exports

```rust
pub use wall::{distance_to_segment, project_onto_segment};
```

(DVec2 is imported directly from `glam` by consumers.)

#### 1d. Rename across codebase

- Replace all `use crate::model::Point2D` / `use super::Point2D` / `use super::wall::Point2D` with `use glam::DVec2`
- Replace all `Point2D::new(x, y)` with `DVec2::new(x, y)`
- Replace all `Point2D { x: ..., y: ... }` struct literals with `DVec2::new(..., ...)`
- Replace all `.distance_to(other)` with `.distance(other)` (~15 sites)
- Replace all `p.distance_to_segment(a, b)` with `distance_to_segment(p, a, b)` (~4 sites)
- Replace all `p.project_onto_segment(a, b)` with `project_onto_segment(p, a, b)` (~5 sites)
- Update type annotations in struct fields: `Point2D` → `DVec2`

### Commit 2: Arithmetic simplification (behavior-preserving)

- Simplify `line_intersection` in `room_metrics.rs` using `perp_dot`
- Simplify `angle_between` in `room_detection.rs` using vector subtraction
- Simplify edge offset computation in `snap.rs` using vector multiply
- Simplify `project_t` in `room_metrics.rs` using `dot`
- Optionally add `fn pos2_to_dvec(p: egui::Pos2) -> DVec2` helper to eliminate the 8 repeated `DVec2::new(world.x as f64, world.y as f64)` casts in `canvas.rs`

## Verification

```bash
cargo build
cargo test
cargo run
# Test: draw walls, verify coordinates are correct
# Test: load existing saved project — should deserialize correctly
# Test: room detection still works
# Test: snap still works
```
