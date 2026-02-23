# Implementation Plan

Based on `docs/todo.md` requirements. Four sessions, ordered by dependency.

---

## Session 1: Snap System Fixes, Snap Indicator, and Wall Endpoint Handles (Req 2 + Req 8)

Foundational fixes to the wall-drawing and selection workflow.

### 1a. Junction creation asymmetry (Req 2, bullet 1)

**Problem**: A T-junction is created both when a wall *starts* from a wall-edge snap and when it *ends* there. It should only be created when the **end point** of a new wall lands on a wall's side edge.

**Files**: `src/app/canvas.rs`, `src/history.rs`

- In `canvas.rs`, the `WallToolState::Idle` branch records the snap but proceeds to `Drawing { start }`. Currently on the second click (`WallToolState::Drawing`), `junction_target` is read from `last_snap` — but the first click's snap can also attach.
- Fix: save the snap from the *first* click separately (e.g. `start_snap` field on `WallTool`), and only use `last_snap` (from the second click) as the `junction_target` for `AddWallCommand`. The first-click snap should only attach the start vertex to the existing endpoint (vertex snap) — it must **not** create a junction on the host wall.
- Verify: starting a wall from an existing wall's side should not split that wall into sections.

### 1b. Phantom wall on grid coincidence (Req 2, bullet 2)

**Problem**: When a vertex and a grid point coincide, grid snap can win due to distance tie. Vertex snap must always take priority.

**Files**: `src/editor/snap.rs`

- Current code checks vertex snap within `VERTEX_SNAP_SCREEN_PX` (15px) and grid snap independently. If a vertex sits exactly on a grid point, both distances are ≈0 — but vertex snap's early return should already win.
- The real issue: when confirming the second point of a wall, the snapped position might be subtly different from an existing vertex because of floating-point rounding in grid snap. Fix by adding a final pass: after grid snap is computed, check if any vertex is within a small epsilon (e.g. 1mm world-space) of the grid-snapped position; if so, substitute the vertex position.
- Alternatively, increase `VERTEX_SNAP_SCREEN_PX` to 20px so vertex snap catches cases where grid snap would otherwise win at moderate zoom levels.

### 1c. Snap indicator on canvas (Req 2, bullet 3)

**Problem**: No visual feedback for snap type during wall drawing.

**Files**: `src/app/canvas_draw.rs`

- Add a new method `draw_snap_indicator(&self, painter, rect)` called during wall tool rendering.
- Read `self.editor.wall_tool.last_snap` to get the `SnapResult`.
- Render a colored ring/crosshair at the snapped screen position:
  - `SnapType::Vertex` → green ring (matching start-point color)
  - `SnapType::WallEdge` → yellow ring
  - `SnapType::Grid` → white/gray ring
  - `SnapType::None` → no indicator (Shift held)
- Call this from `show_canvas()` in `canvas.rs` when the wall tool is active.

### 1d. Wall endpoint handles: selection-gated rendering (Req 8)

**Problem**: Green/yellow endpoint circles render for all walls. They should only render when the wall is selected.

**Files**: `src/app/canvas_draw.rs`

- In `draw_walls()`, lines 80–82 draw `start_color` and `end_color` circles unconditionally.
- Move these two `painter.circle_filled(...)` calls inside the `if is_selected { ... }` block (or wrap them with `if is_selected`).

---

## Session 2: Wall Sections Foundation — Implicit Sections, Side Coloring, Selection Highlight, Scrollbars (Req 9 + Req 5)

Establishes the visual and data foundation for sections, which later requirements depend on.

### 2a. Implicit single section (Req 9, bullet 1)

**Problem**: `SideData.sections` is empty when there are no junctions. Code uses `has_sections()` to skip rendering. Every side must always have at least one section.

**Files**: `src/model/wall.rs`

- Change `SideData::new()` to initialize `sections` with one `SectionData` spanning the full side length, matching the side's height_start/height_end.
- Change `remove_junction()`: when junctions become empty, recompute sections to a single entry (instead of clearing).
- Change `has_sections()` → always returns true (or remove it and fix callers).
- Update `recompute_sections()` to handle the 0-junction case (produces 1 section).
- Ensure serde deserialization handles old data with empty `sections` (add a `fn ensure_sections()` post-deserialization fixup, or handle at load time in `project_io.rs`).

### 2b. Distinct per-side section coloring (Req 9, bullet 2)

**Problem**: Section colors are currently per-index (same palette for both sides) and only shown when selected.

**Files**: `src/app/canvas_draw.rs`

- Remove the `if is_selected` guard around section rendering (lines 104–158). Sections should always be visible.
- Use distinct tint palettes for left side (e.g. blue-ish tints) and right side (e.g. orange-ish tints).
- Render section fill strips as semi-transparent rectangles along each side of the wall body (between the wall outline and centerline), not just a thin colored line.
- Keep junction tick marks visible for walls that have junctions.

### 2c. Selection highlight via outline stroke (Req 9, bullet 3)

**Problem**: Selected walls are flood-filled blue, hiding section coloring underneath.

**Files**: `src/app/canvas_draw.rs`

- Remove the blue fill override for selected walls (lines 26–29 where `fill` changes based on `is_selected`).
- All walls use the same base fill color (the neutral gray `wall_fill`).
- For selected walls, draw an additional outline stroke (thicker, bright color) around the entire wall polygon after the normal rendering pass.
- Section tint colors remain visible underneath.

### 2d. Section labels for all sections including implicit (Req 9, bullet 4)

This is partially addressed by Req 1 (Session 3), but the foundation is: section labels must appear even when there is only one implicit section. The current `has_sections()` guard (line 115) skips label rendering for single-section walls. After 2a makes sections always present, this guard will be removed, and labels will appear for all walls.

### 2e. Scrollbars for side panels (Req 5)

**Problem**: Properties panel and services panel overflow without scrollbars.

**Files**: `src/app/properties_panel.rs`, `src/app/services_panel.rs`

- Wrap the body of `show_right_panel()` content inside `egui::ScrollArea::vertical().show(ui, |ui| { ... })`.
- Similarly wrap the left panel (services panel) content.
- Keep panel headings outside the scroll area if desired for sticky headers.

---

## Session 3: Canvas Labels and Section Length Editing (Req 1 + Req 6)

### 3a. Section dimension labels (Req 1, first part)

**Problem**: Each section should show `{length} - {area}` parallel to the wall centerline, centered on the section. Currently only whole-wall area is shown.

**Files**: `src/app/canvas_draw.rs`

- Replace the current per-wall area labels (lines 160–202) with per-section labels.
- For each side, iterate over sections. For each section:
  - Compute the section's midpoint along the wall (using boundary t-values).
  - Offset the label position to the appropriate side (left or right of centerline).
  - Format: `{length_mm} - {area_m2} м²` (e.g. "3500 - 9.45 м²").
  - Rotate/orient the label parallel to the wall centerline using `painter.text()` with appropriate anchoring (egui doesn't support rotated text natively — use the midpoint position and accept horizontal text, or compute angle and use `galley` with rotation transform if feasible).
- If egui rotation is impractical, draw labels at the section midpoint with horizontal alignment but offset to the correct side.

### 3b. Wall thickness label (Req 1, second part)

**Problem**: Wall thickness should be rendered at the wall center, replacing the current baseline-length label.

**Files**: `src/app/canvas_draw.rs`

- The current label at lines 84–102 shows wall centerline length ("3.50 м"). Replace this with the wall thickness label (e.g. "200 мм" or "0.20 м").
- Position it at the wall body center (midpoint of centerline), parallel to the wall.
- Wall length information is now captured per-section in the section labels (3a).

### 3c. Section length editing in properties panel (Req 6, first part)

**Problem**: `SectionData.length` is read-only in the properties panel. It must be individually editable via a numeric input.

**Files**: `src/app/property_edits.rs`, `src/app/properties_panel.rs`

- In `show_side_sections()`, change each section's "Длина:" row from a label to a `DragValue` input.
- Editing a section's length should update `section.length` directly.
- The section area updates automatically (computed from length × heights).
- Add undo/redo support: track section edits in the wall snapshot mechanism (already captures `SideData` which includes `sections`).

### 3d. Side length locking when junctions exist (Req 6, second part)

**Problem**: When a side has junctions, the total side length should become read-only and computed from: sum of section lengths + thicknesses of all walls creating junctions on that side.

**Files**: `src/app/properties_panel.rs`, `src/model/wall.rs`

- Add a method `SideData::computed_total_length(&self, walls: &[Wall]) -> f64` that sums section lengths and junction wall thicknesses.
- In `show_right_panel()`, when rendering the left/right side length field:
  - If `side.junctions.is_empty()` → render an editable `DragValue` (current behavior).
  - If junctions exist → render a read-only label showing the computed total.
- Update `SideData` to recompute `self.length` from sections + junction thicknesses whenever sections change.

---

## Session 4: Room System — Validity, Section-Based Area, Two Area Values (Req 3 + Req 4 + Req 7)

### 4a. Room validity: require closed contours (Req 3)

**Problem**: Rooms with open contours (from wall deletion/modification) remain in `Project.rooms`.

**Files**: `src/app/canvas.rs` (or `src/app/mod.rs`), `src/editor/room_detection.rs`

- After `merge_rooms()` in `show_canvas()`, add a validation pass: for each existing room, verify its wall list forms a closed contour:
  - All `wall_ids` must reference existing walls.
  - Consecutive walls in the contour must share an endpoint (within epsilon).
  - The last wall must connect back to the first.
- Remove rooms that fail validation from `Project.rooms` (and clean up `room_services`).
- Alternative: `merge_rooms()` already replaces rooms each frame from `WallGraph::detect_rooms()`. Verify this is sufficient — if a wall is deleted, the room should disappear naturally from the next detection cycle. If not, add explicit cleanup.

### 4b. Room area from section lengths (Req 4, main computation)

**Problem**: Current area uses Shoelace formula on the offset polygon (centerline ± half-thickness). New approach: use interior section lengths from `SideData.sections` on the room-facing side.

**Files**: `src/editor/room_metrics.rs`

- For each wall segment in the room contour, take the room-facing side's sections to get the interior measurement.
  - Sum all section lengths on the room-facing side to get the interior wall length for that segment.
  - This accounts for junctions (wall thickness subtracted at T-junction points).
- Build an interior polygon using these section-based lengths:
  - Use the same offset-intersection approach as current code, but replace the offset distance with a per-segment interior length.
  - Or: walk the contour, placing each segment at its interior length and turning by the measured interior angle.
- Compute area of the resulting polygon via Shoelace formula.

### 4c. Corner area correction (Req 4, corner term)

**Files**: `src/editor/room_metrics.rs`

- At each interior corner, approximate the angle between consecutive walls.
- Add or subtract a corner correction term based on the angle and wall thicknesses:
  - For a right angle (90°): the correction is typically `thickness₁ × thickness₂ / 4` (quarter of the overlap rectangle).
  - For other angles: use the appropriate geometric formula.
- This accounts for the overlap or gap at corners.

### 4d. Column-wall exception (Req 4, column case)

**Files**: `src/editor/room_metrics.rs`

- Detect when both sides of a wall face the same room interior (wall acts as a column/partition within the room).
- For such walls, subtract the column's cross-section area (`length × thickness`) from the room area.

### 4e. Room properties: two area values (Req 7)

**Problem**: Properties panel shows one area value. Must show gross area and net area.

**Files**: `src/app/properties_panel.rs`, `src/editor/room_metrics.rs`

- Extend `RoomMetrics` to include both `gross_area` (full bounding polygon including wall volume and window reveals) and `net_area` (clear interior floor area from section-length-based polygon).
- `gross_area`: area of the polygon formed by wall centerlines (no inward offset) — or the outer edges.
- `net_area`: area from the section-length-based interior polygon (from 4b).
- In `show_right_panel()` for `Selection::Room`, display both values:
  ```
  Площадь (брутто): X.XX м²
  Площадь (нетто):  Y.YY м²
  ```
- Update the canvas room label (`draw_rooms()`) to show net area (or both).
