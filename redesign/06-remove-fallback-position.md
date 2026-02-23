# Task 06: Remove fallback_position from Opening model

**Phase:** 1 (model cleanup)
**Estimated savings:** ~10 lines
**Depends on:** Nothing (but easier after Task 07 since fewer mutation sites)

## Goal

Remove the `fallback_position: Option<Point2D>` field from `Opening`. This field stores transient rendering state (position when an opening is dragged off a wall) in the serialized model. Move it to `EditorState` where it belongs.

**Scope reduction from original proposal:** The original Task 06 also proposed making `wall_id` non-optional (`Uuid` instead of `Option<Uuid>`). This has been dropped because `wall_id = None` is NOT transient-only — `RemoveWallCommand::execute` (`history.rs:129`) deliberately sets it to `None` with a `fallback_position` so orphaned openings render as red warnings. Making `wall_id` non-optional would require changing wall-deletion behavior (a UX change) and would break existing save files containing `wall_id: null`.

## Code Review

### Current `Opening` struct (`src/model/opening.rs:64-76`)

```rust
pub struct Opening {
    pub id: Uuid,
    pub kind: OpeningKind,
    pub wall_id: Option<Uuid>,           // KEEP — used for orphaned openings after wall delete
    pub offset_along_wall: f64,
    pub fallback_position: Option<Point2D>,  // REMOVE — transient rendering state
}
```

### Where `fallback_position` is written

1. **`canvas.rs:361`** — during opening drag when cursor leaves all walls. Computes position from the old wall, stores it, then sets `wall_id = None`.
2. **`history.rs:124`** — in `RemoveWallCommand::execute`, computes position from the wall being deleted.

### Where `fallback_position` is read

1. **`canvas_draw.rs:459`** — `opening.fallback_position.unwrap_or(Point2D::new(0.0, 0.0))` for rendering detached openings at their last known position.

### What stays unchanged

`wall_id: Option<Uuid>` stays. `Opening::new()` signature stays. The 12 call sites that unwrap `wall_id` stay as-is. The `OpeningKind` enum stays. The `default_door()` / `default_window()` / `new_door()` / `new_window()` convenience methods stay.

### Rendering consistency concern

When an opening is dragged off a wall, the current code sets `wall_id = None` in the same mutation as `fallback_position`. With the new approach, the opening keeps its old `wall_id` while visually rendering at the detached position. This means quantity computation still counts the opening on the old wall during drag. This is acceptable because:
- The drag is transient (sub-second)
- Quantities are not displayed during drag
- On drag end (release), either the opening is re-attached to a wall (correct state) or it stays detached via `wall_id = None` (same as before, minus fallback_position)

For the `RemoveWallCommand` case: the command already handles `wall_id = None` correctly. The rendering of orphaned openings after wall deletion needs to compute the position from the stored wall data in the command's snapshot, or we compute it once and store it on `EditorState`.

## Changes

### 1. Remove `fallback_position` from `Opening` in `src/model/opening.rs`

```rust
pub struct Opening {
    pub id: Uuid,
    pub kind: OpeningKind,
    pub wall_id: Option<Uuid>,
    pub offset_along_wall: f64,
    // fallback_position removed
}
```

Remove from `Opening::new()` as well.

### 2. Add transient detach state to `EditorState` in `src/editor/mod.rs`

```rust
pub struct EditorState {
    // ... existing fields ...
    /// Transient: screen position for openings with wall_id=None (orphaned or being dragged off).
    /// Maps opening ID to last known world position.
    pub orphan_positions: HashMap<Uuid, Point2D>,
}
```

### 3. Update `canvas.rs` drag handling

When opening is dragged off wall (currently line 361):
- Compute position from old wall
- Store in `self.editor.orphan_positions.insert(oid, computed_pos)`
- Still set `opening.wall_id = None` and remove from old wall's openings list

### 4. Update `history.rs` RemoveWallCommand

When a wall is deleted and openings are orphaned (line 124):
- Compute fallback position from the wall being deleted
- Store in `self.editor.orphan_positions` (requires passing editor state, or computing at render time)

Alternative: compute the position lazily at render time in `canvas_draw.rs` — if `wall_id` is `None` and no orphan position is stored, the opening simply doesn't render. This is simpler and acceptable since the orphan state is transient (undo restores the wall).

### 5. Update `canvas_draw.rs:452-486` — detached rendering

Replace `opening.fallback_position.unwrap_or(...)` with a lookup in `self.editor.orphan_positions`:

```rust
None => {
    let pos = match self.editor.orphan_positions.get(&opening.id) {
        Some(p) => *p,
        None => continue, // no position known, skip rendering
    };
    // ... same rendering code using pos ...
}
```

## Verification

```bash
cargo build
cargo test
cargo run
# Test: place a door on wall, drag it off, drag it back
# Test: delete a wall with openings, verify orphan rendering
# Test: undo wall deletion, verify openings re-attach
# Test: save/load project with openings
```
