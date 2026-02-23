# Task 07: Replace Command pattern with snapshot-based undo

**Phase:** 2 (core redesign, foundational — must be done before Task 08)
**Estimated savings:** ~470 lines
**Depends on:** Task 01 (extract methods makes this easier but not strictly required)
**Blocks:** Task 08 (mutation functions)

## Goal

Replace the 457-line `history.rs` (11 command types + Command trait + WallProps/LabelProps snapshot types) and the 113-line deferred edit machinery in `property_edits.rs` with a ~50-line snapshot-based `History` struct. Eliminate the `push_already_applied()` hack and the `flush_property_edits()` pattern (called 15 times across 3 files).

## Code Review

### `src/history.rs` (457 lines) — FULL REPLACEMENT

- `Command` trait (3 methods: `execute`, `undo`, `description`)
- 9 command structs: Add/Remove/Modify × Wall/Opening/Label
- `WallProps`, `LabelProps` snapshot types
- `History` struct with `push()`, `push_already_applied()`, `undo()`, `redo()`

Note: `description()` is defined on every command but **never displayed in the UI**. No tooltip, no status bar, no undo history panel uses it.

### `src/app/property_edits.rs` — PARTIAL REMOVAL

**Remove (~113 lines):**
- `opening_kind_changed()` (lines 8-23) — epsilon comparison
- `update_edit_snapshots()` (lines 26-53) — selection change detection + flush
- `flush_property_edits()` (lines 55-125) — compares against snapshot, creates Modify commands

**Keep:**
- `has_validation_errors()`, `opening_errors()`, `selection_target_type()`, `show_side_sections()`

**Migration item:** `flush_property_edits()` line 101-107 contains label auto-delete logic (empty text → remove label). This side-effect must be relocated — e.g., to the properties panel or a post-edit validation step.

### `src/app/mod.rs` — REMOVE SNAPSHOT FIELDS

Remove from `App` struct: `wall_edit_snapshot`, `opening_edit_snapshot`, `label_edit_snapshot`.
Remove inits from `new()`, `open_project_from_path()`, `create_new_project()`.
Remove `self.update_edit_snapshots()` from `update()`.

### Call sites for `flush_property_edits()` — ALL REMOVED

15 calls across 3 files: `canvas.rs` (7), `toolbar.rs` (4), `property_edits.rs` (3).

### Memory cost analysis

A typical project (~20 walls, ~10 openings, ~6 rooms): **~17KB per snapshot**.
Large project (~100 walls, ~50 openings, ~20 rooms): **~85KB per snapshot**.
100 snapshots cap: **1.7MB typical, 8.5MB worst case**. Negligible on any modern hardware.

`PriceList` is NOT inside `Project` — stored separately on `App`. Not cloned.

### Canvas drag operations — pre-existing bug

Label drag (`canvas.rs:259-274`) and opening drag (`canvas.rs:275-376`) mutate the project directly with no snapshot and no command. These are **not undoable in the current system either**. Snapshot undo makes fixing this trivial: add `response.drag_started()` check.

### Alternatives considered

| Approach | Verdict |
|----------|---------|
| Full snapshot undo (VecDeque) | **Selected** — maximally simple, correctness guaranteed, ~50 lines |
| Differential snapshot (JSON patch) | Rejected — solves a non-problem (1.7MB vs ~200KB), dramatically more complex |
| Simplified command pattern (macros) | Rejected — ~200 lines, still per-operation undo logic, bug surface area remains |
| Hybrid (snapshot for props, commands for structure) | Rejected — two mechanisms to synchronize, most complex option |

## Changes

### 1. Rewrite `src/history.rs`

Replace the entire 457-line file with:

```rust
use std::collections::VecDeque;
use crate::model::Project;

pub struct History {
    undo_stack: VecDeque<(Project, &'static str)>,
    redo_stack: VecDeque<(Project, &'static str)>,
    /// Monotonically increasing counter, bumped on every snapshot/undo/redo.
    pub version: u64,
    max_entries: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            version: 0,
            max_entries: 100,
        }
    }

    /// Save current project state before a mutation.
    pub fn snapshot(&mut self, project: &Project, description: &'static str) {
        self.undo_stack.push_back((project.clone(), description));
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
        self.version += 1;
    }

    pub fn undo(&mut self, project: &mut Project) -> bool {
        if let Some((prev, desc)) = self.undo_stack.pop_back() {
            self.redo_stack.push_back((project.clone(), desc));
            *project = prev;
            self.version += 1;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, project: &mut Project) -> bool {
        if let Some((next, desc)) = self.redo_stack.pop_back() {
            self.undo_stack.push_back((project.clone(), desc));
            *project = next;
            self.version += 1;
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }
}
```

Key improvements over the original proposal:
- **`VecDeque`** instead of `Vec` — `pop_front()` for O(1) cap enforcement instead of O(n) `remove(0)`
- **Description strings** — `&'static str` per entry preserves the ability to show "Undo: Add wall" in future UI (zero runtime cost)

### 2. Update `src/app/mod.rs`

- Remove 3 snapshot fields from `App` struct
- Remove snapshot inits from `new()`, `open_project_from_path()`, `create_new_project()`
- Remove `self.update_edit_snapshots()` from `update()`
- Update import to `use crate::history::History;`
- Add field: `edit_snapshot_version: Option<u64>`

### 3. Simplify `src/app/property_edits.rs`

Delete `opening_kind_changed()`, `update_edit_snapshots()`, `flush_property_edits()`.
Keep `has_validation_errors()`, `opening_errors()`, `selection_target_type()`, `show_side_sections()`.

Relocate the label auto-delete logic (empty text check) to `show_label_properties()` in `properties_panel.rs`.

### 4. Handle DragValue undo

Add `edit_snapshot_version: Option<u64>` to `App`. At the top of `show_right_panel()`:

```rust
if self.editor.selection != Selection::None && self.edit_snapshot_version != Some(self.history.version) {
    self.history.snapshot(&self.project, "edit properties");
    self.edit_snapshot_version = Some(self.history.version);
}
```

Takes ONE snapshot when editing starts. DragValue changes accumulate into a single undo step. Identical UX to the current `flush_property_edits()` grouping behavior.

### 5. Update `src/app/canvas.rs`

Replace all `self.flush_property_edits(); self.history.push(Box::new(Cmd { ... }), &mut self.project);` with `self.history.snapshot(&self.project, "description");` followed by direct mutation (inline for now — Task 08 extracts to functions).

Add `drag_started()` snapshot for label and opening drag to fix pre-existing no-undo bug:
```rust
if response.drag_started() {
    self.history.snapshot(&self.project, "drag");
}
```

### 6. Update `src/app/toolbar.rs`

Remove `flush_property_edits()` calls before undo/redo. Undo/redo calls stay:
```rust
self.history.undo(&mut self.project);
```

### 7. Update `src/app/properties_panel.rs`

Remove snapshot initialization blocks (lines 23-31, 196-200, 400-408).

## Verification

```bash
cargo build
cargo test
cargo run
# Test: draw a wall, undo (Ctrl+Z), redo (Ctrl+Y)
# Test: select wall, change thickness via DragValue, undo → should restore old thickness
# Test: add opening, delete it, undo → should restore
# Test: chain-draw walls, undo multiple times → each wall undone separately
# Test: drag a label, undo → should restore position (NEW behavior)
```
