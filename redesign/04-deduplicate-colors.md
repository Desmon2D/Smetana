# Task 04: Deduplicate SECTION_COLORS palette

**Phase:** 1 (trivial refactor, no dependencies)
**Estimated savings:** ~7-10 lines
**Depends on:** Nothing

## Goal

The `SECTION_COLORS` constant is defined identically in 3 files. Move it to a single shared location.

## Code Review

### Identical definitions in 3 files

**`src/app/canvas_draw.rs:64-71`** — local `const` inside `draw_walls()` method body.

**`src/app/property_edits.rs:190-197`** — local `const` inside `show_side_sections()` associated function body.

**`src/app/services_panel.rs:111-118`** — local `const` inside `show_wall_side_services()` method body.

All three are byte-identical 6-tuple arrays of `(u8, u8, u8)` with identical usage pattern:
```rust
let global_idx = color_offset + i;
let color_idx = global_idx % SECTION_COLORS.len();
let (cr, cg, cb) = SECTION_COLORS[color_idx];
let color = egui::Color32::from_rgb(cr, cg, cb);
```

### Visibility analysis

All three consumer files are submodules of `app/`. In Rust, private items in a parent module are visible to child modules. A bare `const` (no `pub` qualifier) in `app/mod.rs` is visible to all submodules via `super::SECTION_COLORS`. No need for `pub(super)`.

### What NOT to extract

The `blend_color` closure in `canvas_draw.rs:74-81` blends a palette color with neutral gray. It is used **only once** in `canvas_draw.rs:175`. Extracting a single-use closure to a module-level function in a different file reduces code locality for zero deduplication benefit. **Leave it as a local closure.**

### Alternatives considered

| Approach | Verdict |
|----------|---------|
| Private `const` in `app/mod.rs` | **Selected** — minimal change, follows existing conventions |
| New `app/colors.rs` module | Rejected — introducing a module for one 6-line constant is overhead without proportional benefit |
| `section_color(idx) -> Color32` helper function | Rejected — good long-term design but exceeds scope of "trivial refactor", changes API surface |

### Other duplicated colors noted (for future, not this task)

- `wall_fill = (140, 140, 145)` appears in `canvas_draw.rs:50` and `wall_joints.rs:96` (crosses module boundary)
- `start_color = (60, 200, 80)` appears in 3 places within `app/`
- These are separate, differently-scoped tasks

## Changes

### 1. Add shared constant to `src/app/mod.rs`

Add after imports, before struct definitions:

```rust
/// Section color palette shared across canvas rendering, property editors, and services panel.
const SECTION_COLORS: &[(u8, u8, u8)] = &[
    (100, 180, 240),
    (240, 160, 100),
    (100, 220, 140),
    (220, 120, 220),
    (240, 220, 100),
    (120, 220, 220),
];
```

Note: no `pub` qualifier. Submodules access via `super::SECTION_COLORS`.

### 2. Remove local definitions

- **`canvas_draw.rs`:** Delete lines 64-71. Add `use super::SECTION_COLORS;` at the top (alongside existing `use super::App;`). Leave `blend_color` closure as-is.
- **`property_edits.rs`:** Delete lines 190-197 inside `show_side_sections()`. Reference as `super::SECTION_COLORS` or add import.
- **`services_panel.rs`:** Delete lines 111-118 inside `show_wall_side_services()`. Add to existing import: `use super::{App, ServiceTarget, SECTION_COLORS};`.

## Verification

```bash
cargo build
cargo run  # visual check: section colors should be identical
```
