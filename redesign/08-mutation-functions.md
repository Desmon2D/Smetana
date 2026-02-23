# Task 08: Extract mutation functions on Project

**Phase:** 2 (core redesign, after snapshot undo)
**Estimated savings:** ~30-50 lines (deduplication of inline mutation logic)
**Depends on:** Task 07 (snapshot undo)

## Goal

Extract the mutation logic scattered across `canvas.rs` into reusable methods on `Project`. This consolidates:
- 3 nearly identical delete blocks into one `delete_selected()` method on `App`
- Junction registration logic (duplicated between AddWallCommand::execute and canvas.rs) into `Project::add_wall()`
- Opening linkage/unlinkage into `Project::add_opening()` / `Project::remove_opening()`
- Service mutation consolidation

This replaces the original Task 05 proposal (Action enum + centralized event processing), which was rejected for the following reasons:
- **Correctness bug:** `MoveOpening` with `needs_snapshot() -> true` would snapshot on every drag frame at 60fps
- **Borrow checker is not the driver:** the current code already mutates `self.project` inside egui closures — no technical need to defer mutations
- **Net line count:** the Action enum + dispatch loop + `&mut Vec<Action>` parameter threading added more lines than it saved
- **Two conflicting mutation patterns:** actions for discrete events + direct mutation for DragValues would confuse LLMs

## Code Review

### Mutation logic currently inline in `canvas.rs`

After Task 07 replaces command pattern with snapshot undo, the mutation logic from command `execute()` bodies moves inline into canvas.rs. This creates duplication:

**Add wall + junctions (2 places, ~20 lines each):**
- Lines 114-124 (closing wall in chain)
- Lines 145-154 (normal wall / chained wall)

Both do: register junction on target wall side → push wall → set selection.

**Delete selected (3 blocks, ~8 lines each):**
- Lines 383-391 (wall: remove openings, clean junctions, remove wall)
- Lines 393-399 (opening: unlink from wall, remove)
- Lines 401-407 (label: remove)

**Add opening (lines 459-467):**
Link to wall's openings list → push opening → set selection.

**Add label (lines 475-483):**
Push label → set selection.

### Service mutations in `services_panel.rs`

**Remove service (2 places, ~12 lines each, lines 184-195 and 246-252):**
Nearly identical match on `ServiceTarget` variant.

**Update custom price (2 places, ~20 lines each, lines 197-222 and 254-272):**
Nearly identical match + price update logic.

### The `dirty` flag problem

12 scattered `self.dirty = true` assignments. Most are for service operations and room name edits that don't go through history. Solution: add `History::mark_dirty()` that bumps `version` without storing a snapshot. Then `auto_save()` only checks `version != last_saved_version`, and the `dirty` field can be removed entirely.

## Changes

### 1. Add mutation methods to `Project` in `src/model/project.rs`

```rust
impl Project {
    /// Add a wall, registering T-junctions on target walls.
    pub fn add_wall(
        &mut self,
        wall: Wall,
        junction_target: Option<(Uuid, WallSide, f64)>,
        start_junction_target: Option<(Uuid, WallSide, f64)>,
    ) {
        for jt in [&junction_target, &start_junction_target] {
            if let Some((target_id, side, t)) = jt {
                if let Some(target) = self.walls.iter_mut().find(|w| w.id == *target_id) {
                    let sd = match side {
                        WallSide::Left => &mut target.left_side,
                        WallSide::Right => &mut target.right_side,
                    };
                    sd.add_junction(wall.id, *t);
                }
            }
        }
        self.walls.push(wall);
    }

    /// Remove a wall, its attached openings, and junction references from other walls.
    pub fn remove_wall(&mut self, id: Uuid) {
        self.openings.retain(|o| o.wall_id != Some(id));
        for w in &mut self.walls {
            w.left_side.remove_junction(id);
            w.right_side.remove_junction(id);
        }
        self.walls.retain(|w| w.id != id);
    }

    /// Add an opening, linking it to its wall.
    pub fn add_opening(&mut self, opening: Opening) {
        if let Some(wid) = opening.wall_id {
            if let Some(wall) = self.walls.iter_mut().find(|w| w.id == wid) {
                wall.openings.push(opening.id);
            }
        }
        self.openings.push(opening);
    }

    /// Remove an opening, unlinking it from its wall.
    pub fn remove_opening(&mut self, id: Uuid) {
        if let Some(opening) = self.openings.iter().find(|o| o.id == id) {
            if let Some(wid) = opening.wall_id {
                if let Some(wall) = self.walls.iter_mut().find(|w| w.id == wid) {
                    wall.openings.retain(|oid| *oid != id);
                }
            }
        }
        self.openings.retain(|o| o.id != id);
    }

    /// Remove a label by ID.
    pub fn remove_label(&mut self, id: Uuid) {
        self.labels.retain(|l| l.id != id);
    }
}
```

### 2. Add `delete_selected()` to `App` in `src/app/mod.rs`

Consolidates 3 delete blocks into one method:

```rust
fn delete_selected(&mut self) {
    match self.editor.selection {
        Selection::Wall(id) => {
            self.history.snapshot(&self.project, "delete wall");
            self.project.remove_wall(id);
        }
        Selection::Opening(id) => {
            self.history.snapshot(&self.project, "delete opening");
            self.project.remove_opening(id);
        }
        Selection::Label(id) => {
            self.history.snapshot(&self.project, "delete label");
            self.project.remove_label(id);
        }
        _ => return,
    }
    self.editor.selection = Selection::None;
}
```

### 3. Simplify `canvas.rs` call sites

**Add wall (2 places → 2 places but shorter):**
```rust
self.history.snapshot(&self.project, "add wall");
let wall_id = wall.id;
self.project.add_wall(wall, junction_target, start_junction_target);
self.editor.selection = Selection::Wall(wall_id);
```

**Delete (3 blocks → 1 call):**
```rust
if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
    self.delete_selected();
}
```

**Add opening:**
```rust
self.history.snapshot(&self.project, "add opening");
let oid = opening.id;
self.project.add_opening(opening);
self.editor.selection = Selection::Opening(oid);
```

**Add label:**
```rust
self.history.snapshot(&self.project, "add label");
let lid = label.id;
self.project.labels.push(label);
self.editor.selection = Selection::Label(lid);
```

### 4. Optionally eliminate `dirty` field

Add to `History`:
```rust
/// Bump version without storing a snapshot. For non-undoable state changes (service edits, room renames).
pub fn mark_dirty(&mut self) {
    self.version += 1;
}
```

Replace all `self.dirty = true` with `self.history.mark_dirty()`. Remove the `dirty` field from `App`. Update `auto_save()` to only check `self.history.version != self.last_saved_version`.

### 5. Consolidate service mutations in `services_panel.rs`

The two `RemoveService` blocks and two `UpdatePrice` blocks share identical match-on-ServiceTarget structure. Extract helper methods:

```rust
fn remove_service_at(&mut self, target: &ServiceTarget, index: usize) { ... }
fn update_service_price(&mut self, target: &ServiceTarget, index: usize, price: Option<f64>) { ... }
```

## Verification

```bash
cargo build
cargo test
cargo run
# Test all mutation paths: add/delete walls, openings, labels
# Test undo/redo of each
# Test service assignment and removal
# Test opening drag between walls
# Test auto-save triggers correctly
```
