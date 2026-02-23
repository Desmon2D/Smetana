# Task 01: Extract helper methods from monolithic functions

**Phase:** 0 (do first — enables all subsequent tasks)
**Estimated savings:** 0 lines (structural improvement only)
**Depends on:** Nothing
**Blocks:** Makes Tasks 07, 08 easier to implement

## Goal

Decompose the 530-line `show_canvas()` and 430-line `show_right_panel()` into named helper methods within their existing files. This makes each tool handler and property panel independently modifiable — critical for the snapshot undo (Task 07) and mutation function (Task 08) refactors.

No new files, no new directories, no module changes. Just method extraction.

## Code Review

### `src/app/canvas.rs` (545 lines) — single `show_canvas()` method

Internal structure (all inlined in one method body):
- Lines 10-44: Canvas setup (pan, zoom, grid, cursor) — 34 lines
- Lines 46-168: Wall tool state machine (snap, click, chain) — 122 lines
- Lines 171-411: Select tool (hit detection, drag opening, drag label, delete) — 240 lines
- Lines 414-468: Door/Window tool (hover detection, placement click) — 54 lines
- Lines 471-486: Label tool (click to place) — 15 lines
- Lines 488-541: Room detection, draw calls, status bar — 53 lines

The tool sections share `response`, `rect`, `shift_held`, `space_held` from the outer scope. These become method parameters.

### `src/app/properties_panel.rs` (452 lines) — single `show_right_panel()` method

Internal structure (inside one match on `self.editor.selection`):
- Lines 18-21: `Selection::None` — 3 lines
- Lines 22-193: `Selection::Wall(id)` — 171 lines (SideInfo pre-compute, thickness/length/sections/services)
- Lines 195-329: `Selection::Opening(id)` — 134 lines (errors, dimensions, services)
- Lines 331-397: `Selection::Room(id)` — 66 lines (metrics, name edit, services)
- Lines 399-447: `Selection::Label(id)` — 48 lines (text, font size, rotation)

Each match arm can be extracted to a separate private method on `App`.

### `src/app/canvas_draw.rs` (823 lines) — already well-structured

This file already has 6 independent `pub(super)` methods. No extraction needed — it is the lowest priority. The methods are genuinely independent `&self` readers. Leave as-is unless post-refactor size is still deemed problematic.

## Changes

### 1. Extract tool handlers from `show_canvas()` in `src/app/canvas.rs`

Replace the inlined tool logic with dispatch:

```rust
pub(super) fn show_canvas(&mut self, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = response.rect;

        // Pan/zoom/grid setup (stays inline, ~30 lines)
        // ...

        let shift_held = ui.input(|i| i.modifiers.shift);
        let space_held = ui.input(|i| i.key_down(egui::Key::Space));

        match self.editor.active_tool {
            EditorTool::Wall => self.handle_wall_tool(&response, rect, shift_held, space_held),
            EditorTool::Select => self.handle_select_tool(ui, &response, rect, shift_held, space_held),
            EditorTool::Door | EditorTool::Window => self.handle_opening_tool(&response, rect, space_held),
            EditorTool::Label => self.handle_label_tool(&response, rect, space_held),
        }

        // Room detection + draw calls + status bar (stays inline, ~50 lines)
        // ...
    });
}
```

Create 4 private methods in the same file:

- `fn handle_wall_tool(&mut self, response: &egui::Response, rect: egui::Rect, shift_held: bool, space_held: bool)` — lines 46-168
- `fn handle_select_tool(&mut self, ui: &egui::Ui, response: &egui::Response, rect: egui::Rect, shift_held: bool, space_held: bool)` — lines 171-411
- `fn handle_opening_tool(&mut self, response: &egui::Response, rect: egui::Rect, space_held: bool)` — lines 414-468
- `fn handle_label_tool(&mut self, response: &egui::Response, rect: egui::Rect, space_held: bool)` — lines 471-486

### 2. Extract property panels from `show_right_panel()` in `src/app/properties_panel.rs`

Replace the inlined match arms with dispatch:

```rust
pub(super) fn show_right_panel(&mut self, ctx: &egui::Context) {
    egui::SidePanel::right("right_panel").default_width(250.0).show(ctx, |ui| {
        ui.heading("Свойства");
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.editor.selection {
                Selection::None => { ui.label("Ничего не выбрано"); }
                Selection::Wall(id) => self.show_wall_properties(ui, id),
                Selection::Opening(id) => self.show_opening_properties(ui, id),
                Selection::Room(id) => self.show_room_properties(ui, id),
                Selection::Label(id) => self.show_label_properties(ui, id),
            }
        });
    });
}
```

Create 4 private methods in the same file:

- `fn show_wall_properties(&mut self, ui: &mut egui::Ui, id: Uuid)` — lines 22-193
- `fn show_opening_properties(&mut self, ui: &mut egui::Ui, id: Uuid)` — lines 195-329
- `fn show_room_properties(&mut self, ui: &mut egui::Ui, id: Uuid)` — lines 331-397
- `fn show_label_properties(&mut self, ui: &mut egui::Ui, id: Uuid)` — lines 399-447

### 3. No changes to `canvas_draw.rs`

Already well-structured with 6 independent methods. Leave as-is.

## Why do this first

- Task 07 (snapshot undo) needs to replace `flush_property_edits() + history.push()` patterns at 7 sites in canvas.rs. With extracted methods, each site is in a named, isolated function.
- Task 08 (mutation functions) needs to replace inline mutation blocks with `self.project.remove_wall(id)` calls. Isolated methods make this a clear per-method refactor.
- No risk of conflict with any subsequent task.

## Verification

```bash
cargo build
cargo test
cargo run
# Verify all tools work: wall draw, select, door, window, label
# Verify all property panels display correctly
# Verify room detection still triggers
```
