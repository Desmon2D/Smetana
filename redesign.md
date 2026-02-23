# Smetana Redesign Plan

Goal: reduce codebase size for LLM context efficiency while preserving all functionality.

Total estimated savings: **~670 lines** across 8 tasks.

## Implementation Order

Tasks are ordered by dependency and phase. Tasks within the same phase can be done in any order unless noted.

### Phase 0 — Structural prerequisite

| # | Task | Savings | File |
|---|------|---------|------|
| 01 | [Extract helper methods from monolithic functions](redesign/01-extract-methods.md) | 0 lines (structural) | canvas.rs, properties_panel.rs |

Decompose `show_canvas()` (530 lines) and `show_right_panel()` (430 lines) into named helper methods within existing files. No new files or modules. Enables Tasks 07 and 08.

### Phase 1 — Drop-in replacements and trivial refactors (independent, any order)

| # | Task | Savings | File |
|---|------|---------|------|
| 02 | [Replace triangulation with `earcutr`](redesign/02-earcutr-triangulation.md) | ~110 lines | triangulation.rs |
| 03 | [Replace date formatting with `chrono`](redesign/03-chrono-dates.md) | ~40 lines | project_list.rs |
| 04 | [Deduplicate SECTION_COLORS palette](redesign/04-deduplicate-colors.md) | ~7-10 lines | canvas_draw.rs, property_edits.rs, services_panel.rs |
| 05 | [Replace Point2D with glam::DVec2](redesign/05-glam-math.md) | ~20-30 lines | 12 files |

### Phase 2 — Core redesign (sequential: 06 before 07 before 08)

| # | Task | Savings | File |
|---|------|---------|------|
| 06 | [Remove fallback_position from Opening](redesign/06-remove-fallback-position.md) | ~10 lines | opening.rs, canvas.rs, editor/mod.rs |
| 07 | [Replace Command pattern with snapshot undo](redesign/07-snapshot-undo.md) | ~470 lines | history.rs, property_edits.rs, canvas.rs, toolbar.rs |
| 08 | [Extract mutation functions on Project](redesign/08-mutation-functions.md) | ~30-50 lines | project.rs, canvas.rs, app/mod.rs, services_panel.rs |

Task 07 is the highest-impact change (eliminates 457-line history.rs + 113-line deferred edit machinery). Task 08 depends on 07 because it consolidates the inline mutation logic that 07 creates.

## Key Design Decisions

- **Snapshot undo over command pattern**: `VecDeque<(Project, &'static str)>` replaces 11 command types. ~17KB per snapshot typical, 1.7MB at 100-entry cap.
- **Mutation functions over action enum**: Direct methods on `Project` (`add_wall`, `remove_wall`, etc.) instead of `Action` enum + dispatch loop. The borrow checker doesn't force deferred mutations in egui.
- **Full DVec2 rename, no type alias**: `type Point2D = DVec2` creates two-name confusion. Mechanical rename across ~65 references.
- **Method extraction, not file splitting**: Extracting helper methods within existing files instead of splitting into directory-modules. Proportionate to codebase size.
- **`wall_id: Option<Uuid>` stays**: Not purely transient — `RemoveWallCommand` deliberately sets it to `None` for persistent orphaned openings. Only `fallback_position` is removed.
