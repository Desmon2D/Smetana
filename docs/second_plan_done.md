# Plan: Wall Sides, T-Junctions, and Rendering Improvements

This plan is structured for sequential LLM execution. Each phase is self-contained with exact file paths, code changes, and verification criteria. Execute phases in order — each builds on the previous.

---

## Decisions

1. Side lengths are **always user-editable** in the properties panel.
2. Room area polygon uses **wall side vertices** (offset from centerline by half-thickness toward the room-facing side).
3. **No save migration needed** — drop old format support entirely.
4. Wall area labels: **per-side**, displayed **at all zoom levels**.
5. T-junction endpoint snaps to **the edge of the side surface**.
6. Each **side or section** has its **own services**.
7. For 3+ walls at a junction: **hub polygon** approach.
8. Each section gets a **unique color**, shown **only when the wall is selected**.
9. Wall selection = **whole wall** (Option A). Properties panel shows both sides and their sections.
10. **Services UI rework**: bottom panel removed. Services shown in right panel under properties. Price list in a separate `egui::Window`. Service picker is a separate filterable window.

---

## Phase 1: Wall Side Data Model (Changes 1 + 2) ✅ DONE

### Goal
Add `SideData` to `Wall`. Each wall has left and right sides with independent length, height_start, height_end. Update properties panel and all calculations to use side data.

### Step 1.1 — Add SideData struct ✅
**File:** `src/model/wall.rs`

Add before the `Wall` struct:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideData {
    /// Side length in mm (user-editable)
    pub length: f64,
    /// Height at the start end of the wall (mm)
    pub height_start: f64,
    /// Height at the end end of the wall (mm)
    pub height_end: f64,
}

impl SideData {
    pub fn new(length: f64, height_start: f64, height_end: f64) -> Self {
        Self { length, height_start, height_end }
    }

    /// Gross area in mm² (trapezoid formula)
    pub fn gross_area(&self) -> f64 {
        self.length * (self.height_start + self.height_end) / 2.0
    }
}
```

### Step 1.2 — Add sides to Wall ✅
**File:** `src/model/wall.rs`

Add fields to `Wall` struct:
```rust
pub left_side: SideData,   // left side looking from start to end
pub right_side: SideData,  // right side looking from start to end
```

Update `Wall::new()` to initialize both sides:
```rust
let length = start.distance_to(end);
// ...
left_side: SideData::new(length, 2700.0, 2700.0),
right_side: SideData::new(length, 2700.0, 2700.0),
```

Remove `Wall::gross_area()`. Replace with:
```rust
pub fn left_area(&self) -> f64 { self.left_side.gross_area() }
pub fn right_area(&self) -> f64 { self.right_side.gross_area() }
```

Keep `Wall::length()` — it is used for canvas rendering only.

Remove `height_start` and `height_end` from `Wall` struct — they are now in `SideData`. If any rendering code used them, use `left_side.height_start` or a helper.

### Step 1.3 — Update properties panel ✅
**File:** `src/app.rs` — wall properties section (~line 780)

Replace the current wall properties UI with:
```
Стена
├── Толщина (мм): [DragValue]
├── Длина (графика): X.XX м  (read-only, from Wall::length())
├── ■ Левая сторона        ← blue colored label rgb(100, 160, 220)
│   ├── Длина (мм): [DragValue]      ← editable
│   ├── Высота начала (мм): [DragValue]
│   └── Высота конца (мм): [DragValue]
└── ■ Правая сторона       ← purple colored label rgb(170, 100, 200)
    ├── Длина (мм): [DragValue]
    ├── Высота начала (мм): [DragValue]
    └── Высота конца (мм): [DragValue]
```

Remove the old height_start / height_end / area fields from the wall properties.

### Step 1.4 — Update calculations ✅
**File:** `src/app.rs` — `compute_quantity()` function

Anywhere `wall.gross_area()` was used, determine which side is relevant:
- For services: will be per-side in Phase 6. For now, use `wall.left_area()` as placeholder.
- For room metrics: uses the room-facing side — handled in Phase 5.

**File:** `src/export/excel.rs`

Update gross/net area calculations to use `wall.left_side` and `wall.right_side`. For the "Rooms" sheet, use the room-facing side. For per-wall summary, show both sides.

### Step 1.5 — Update history commands ✅
**File:** `src/history.rs`

`ModifyWallCommand` currently stores `WallProps { thickness, height_start, height_end }`. Replace with:
```rust
struct WallProps {
    thickness: f64,
    left_side: SideData,
    right_side: SideData,
}
```

Update `wall_edit_snapshot` in `app.rs` to capture `SideData` for both sides. Update `flush_property_edits()` to compare and create commands using the new `WallProps`.

### Verification ✅
- `cargo build` compiles without errors.
- Creating a wall sets both sides to default values.
- Properties panel shows both sides with editable fields.
- Changing a side's length in the panel persists.
- Undo/redo of wall property changes works.

---

## Phase 2: Color-Coded Endpoints (Change 4) ✅ DONE

### Goal
Wall start = green circle, end = yellow circle. Matching labels in properties panel.

### Step 2.1 — Update endpoint rendering ✅
**File:** `src/app.rs` — `draw_walls()` method (~line 2067-2070)

Replace:
```rust
let endpoint_color = egui::Color32::from_rgb(200, 200, 220);
// ...
painter.circle_filled(start_screen, ep_radius, endpoint_color);
painter.circle_filled(end_screen, ep_radius, endpoint_color);
```

With:
```rust
let start_color = egui::Color32::from_rgb(60, 200, 80);    // green
let end_color = egui::Color32::from_rgb(230, 210, 50);      // yellow
// ...
painter.circle_filled(start_screen, ep_radius, start_color);
painter.circle_filled(end_screen, ep_radius, end_color);
```

### Step 2.2 — Update properties panel labels ✅
**File:** `src/app.rs` — wall properties section

Add colored labels before the side properties:
```rust
ui.horizontal(|ui| {
    ui.colored_label(egui::Color32::from_rgb(60, 200, 80), "●");
    ui.label("Начало (зелёный)");
});
// ... start-related info if any ...
ui.horizontal(|ui| {
    ui.colored_label(egui::Color32::from_rgb(230, 210, 50), "●");
    ui.label("Конец (жёлтый)");
});
```

### Verification ✅
- Selected and unselected walls show green start and yellow end circles.
- Properties panel shows colored labels.

---

## Phase 3: Unattached Openings (Change 8) ✅ DONE

### Goal
Openings not attached to walls render at their last known position, highlighted in red. Currently they render as a dot at origin.

### Step 3.1 — Add fallback_position to Opening ✅
**File:** `src/model/opening.rs`

Add field to `Opening`:
```rust
/// World position for rendering when not attached to a wall
pub fallback_position: Option<Point2D>,
```

Update `Opening::new_door()` and `Opening::new_window()` (or wherever openings are constructed) to set `fallback_position: None`.

### Step 3.2 — Store position on detach ✅
**File:** `src/app.rs`

Wherever an opening loses its `wall_id` (dragged off wall, or parent wall deleted):
- Before setting `wall_id = None`, compute the opening's world position from the wall:
```rust
let wall_dir_x = wall.end.x - wall.start.x;
let wall_dir_y = wall.end.y - wall.start.y;
let wall_len = wall.length();
if wall_len > 0.0 {
    let t = opening.offset_along_wall / wall_len;
    let pos = Point2D::new(
        wall.start.x + wall_dir_x * t,
        wall.start.y + wall_dir_y * t,
    );
    opening.fallback_position = Some(pos);
}
```

Also check `RemoveWallCommand` in `history.rs` — when a wall is deleted, its openings should get `wall_id = None` and a `fallback_position`.

### Step 3.3 — Render unattached openings ✅
**File:** `src/app.rs` — `draw_openings()`, the `None` branch (~line 2189-2201)

Replace the red dot at origin with:
```rust
None => {
    let pos = opening.fallback_position.unwrap_or(Point2D::new(0.0, 0.0));
    let screen_pos = self.editor.canvas.world_to_screen(
        egui::pos2(pos.x as f32, pos.y as f32), center,
    );
    let (w, h) = match &opening.kind {
        OpeningKind::Door { width, height } => (*width, *height),
        OpeningKind::Window { width, height, .. } => (*width, *height),
    };
    let half_w_screen = (w as f32 * self.editor.canvas.zoom) / 2.0;
    let half_h_screen = (h as f32 * self.editor.canvas.zoom) / 2.0;
    // Red-outlined rectangle
    let rect = egui::Rect::from_center_size(
        screen_pos,
        egui::vec2(half_w_screen * 2.0, half_h_screen * 2.0),
    );
    let red = egui::Color32::from_rgb(220, 50, 50);
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0, red));
    // Warning label
    let label = match &opening.kind {
        OpeningKind::Door { .. } => "⚠ Дверь",
        OpeningKind::Window { .. } => "⚠ Окно",
    };
    painter.text(
        egui::pos2(screen_pos.x, rect.top() - 4.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        egui::FontId::proportional(12.0),
        red,
    );
    continue;
}
```

### Step 3.4 — Make unattached openings selectable ✅
**File:** `src/app.rs` — click detection in select tool

When checking for opening clicks, also check distance to `fallback_position` for unattached openings.

### Verification ✅
- Create an opening on a wall, delete the wall → opening renders at its last position in red.
- The opening is selectable at its fallback position.
- Dragging the opening onto a wall re-attaches it.

---

## Phase 4: Concave Room Rendering (Change 5) ✅ DONE

### Goal
Fix room polygon rendering for concave rooms (L-shaped, etc.). Replace `convex_polygon` with ear-clipping triangulation.

### Step 4.1 — Add triangulation module ✅
**File:** `src/editor/triangulation.rs` (new file)

Implement ear-clipping triangulation:
```rust
/// Triangulate a simple polygon using ear-clipping.
/// Input: vertices in order (CCW or CW).
/// Output: list of triangle index triples [i, j, k] referencing input vertices.
pub fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]>
```

Algorithm:
1. Create a mutable list of vertex indices.
2. Compute signed area to determine winding. If CW, reverse the index list.
3. Loop while indices.len() > 3:
   - For each vertex i in the list, check if it's an "ear":
     - The triangle (prev, i, next) must be convex (cross product > 0 for CCW).
     - No other vertex in the list is inside this triangle.
   - If ear found: add triangle to output, remove i from list.
4. Add the final 3-vertex triangle.
5. Return the triangles.

Helper: `point_in_triangle(p, a, b, c) -> bool` using barycentric coordinates.

### Step 4.2 — Register module ✅
**File:** `src/editor/mod.rs`

Add: `pub mod triangulation;`

### Step 4.3 — Update room rendering ✅
**File:** `src/app.rs` — `draw_rooms()` (~line 2381)

Replace:
```rust
painter.add(egui::Shape::convex_polygon(screen_pts.clone(), fill, egui::Stroke::NONE));
```

With:
```rust
let triangles = crate::editor::triangulation::triangulate(&screen_pts);
for tri in &triangles {
    let tri_pts = vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]];
    painter.add(egui::Shape::convex_polygon(tri_pts, fill, egui::Stroke::NONE));
}
// Outline
painter.add(egui::Shape::closed_line(screen_pts.clone(), egui::Stroke::new(1.0, fill)));
```

### Verification ✅
- Create an L-shaped room (5+ walls forming a concave polygon).
- The room fills correctly without triangles extending outside.
- Simple rectangular rooms still render correctly.

---

## Phase 5: Room Area from Wall Sides (Change 3) ✅ DONE

### Goal
Room area polygon uses wall side edges (offset from centerline by half-thickness toward the room). Perimeter uses room-facing `side.length`.

### Step 5.1 — Update compute_room_metrics ✅
**File:** `src/editor/room_detection.rs` — `compute_room_metrics()` (~line 299)

The current code already offsets by half-thickness toward the room-facing side — this is geometrically correct. The change is:
1. Keep the polygon computation as-is (offset centerline by half-thickness on the room-facing side).
2. Change **perimeter** calculation: instead of summing polygon edge lengths, sum the room-facing `side.length` values:
```rust
let mut perimeter = 0.0;
for (i, wall_id) in room.wall_ids.iter().enumerate() {
    let wall = walls.iter().find(|w| w.id == *wall_id)?;
    let side = match room.wall_sides[i] {
        WallSide::Left => &wall.left_side,
        WallSide::Right => &wall.right_side,
    };
    perimeter += side.length;
}
```
3. Area stays as Shoelace on the offset polygon (this represents the actual room floor area bounded by wall surfaces).

### Step 5.2 — Update room display ✅
**File:** `src/app.rs` — `draw_rooms()` and room properties

Use `side.length` values in any room perimeter display. Area comes from `RoomMetrics.area` as before.
(No code change needed — app.rs and excel.rs already use `RoomMetrics.perimeter` which now comes from `side.length` sums.)

### Step 5.3 — Update Excel export ✅
**File:** `src/export/excel.rs`

Room perimeter in the export uses `side.length` sum. Room area uses `RoomMetrics.area`. Per-wall data in the room detail section shows the room-facing side's length and area.
(No code change needed — export already uses `compute_room_metrics` which now returns the correct perimeter.)

### Verification ✅
- Room perimeter in properties panel matches sum of room-facing side lengths.
- Room area matches Shoelace formula on offset polygon.
- Excel export shows correct values.

---

## Phase 6: Services UI Rework + Per-Side Services (Change 6) ✅ DONE

### Goal
1. Remove the bottom panel entirely.
2. Assigned services for the selected object appear in the right panel below properties.
3. Price list editor opens in a separate `egui::Window` via a toolbar button "Услуги".
4. Adding a service opens a filterable picker window.
5. Wall services are per-side (and per-section when T-junctions exist, added in Phase 8).

### Step 6.1 — New data model for wall services ✅
**File:** `src/model/project.rs`

Replace:
```rust
pub wall_services: HashMap<Uuid, Vec<AssignedService>>,
```

With:
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SideServices {
    /// One entry per section. If no T-junctions, exactly 1 entry.
    pub sections: Vec<Vec<AssignedService>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WallSideServices {
    pub left: SideServices,
    pub right: SideServices,
}

// In Project:
pub wall_services: HashMap<Uuid, WallSideServices>,
```

Update `Project::new()` accordingly.

### Step 6.2 — Add UI state for windows ✅
**File:** `src/app.rs` — `App` struct fields

Add:
```rust
/// Whether the price list window is open
show_price_list_window: bool,
/// Whether the service picker window is open
show_service_picker: bool,
/// Filter text for the service picker
service_picker_filter: String,
/// Target for service picker: which object/side/section to assign to
service_picker_target: Option<ServiceTarget>,
```

Define `ServiceTarget`:
```rust
enum ServiceTarget {
    WallSide { wall_id: Uuid, side: WallSide, section_index: usize },
    Opening { opening_id: Uuid },
    Room { room_id: Uuid },
}
```

### Step 6.3 — Remove bottom panel ✅
**File:** `src/app.rs`

Remove the entire bottom panel (`egui::TopBottomPanel::bottom`) that contains the "Price List" and "Assigned Services" tabs. Remove `show_assigned_services_tab`, `show_price_list_tab`, and related methods.

### Step 6.4 — Add "Услуги" button to toolbar ✅
**File:** `src/app.rs` — top panel / toolbar

Add a button:
```rust
if ui.button("Услуги").clicked() {
    self.show_price_list_window = !self.show_price_list_window;
}
```

### Step 6.5 — Price list window ✅
**File:** `src/app.rs`

Add a method `show_price_list_window()` that renders an `egui::Window`:
```rust
fn show_price_list_window(&mut self, ctx: &egui::Context) {
    if !self.show_price_list_window { return; }
    egui::Window::new("Список услуг")
        .open(&mut self.show_price_list_window)
        .default_size([500.0, 400.0])
        .show(ctx, |ui| {
            // Filter input
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut self.price_list_filter);
            });
            // Services table: name, target type, unit, price — all editable
            // Add / Delete / Import / Export buttons
            // (Reuse existing price list editing logic from the old bottom panel)
        });
}
```

Call this from `App::update()` after rendering panels.

### Step 6.6 — Services in right panel ✅
**File:** `src/app.rs` — right panel, after properties section

For each selection type, show assigned services below the properties:

**Wall selected:**
```
───── Услуги ─────
■ Левая сторона (синяя)
  Штукатурка | 7.29 м² | 500₽ | 3645₽  [×]
  [+ Добавить услугу]
■ Правая сторона (фиолетовая)
  Покраска | 5.40 м² | 300₽ | 1620₽  [×]
  [+ Добавить услугу]
```

**Opening selected:**
```
───── Услуги ─────
  Установка | 1 шт | 3000₽ | 3000₽  [×]
  [+ Добавить услугу]
```

**Room selected:**
```
───── Услуги ─────
  Укладка пола | 12.5 м² | 800₽ | 10000₽  [×]
  [+ Добавить услугу]
```

The `[+ Добавить услугу]` button sets `service_picker_target` and `show_service_picker = true`.

### Step 6.7 — Service picker window ✅
**File:** `src/app.rs`

Add a method `show_service_picker_window()`:
```rust
fn show_service_picker_window(&mut self, ctx: &egui::Context) {
    if !self.show_service_picker { return; }
    egui::Window::new("Выбор услуги")
        .open(&mut self.show_service_picker)
        .default_size([400.0, 300.0])
        .show(ctx, |ui| {
            // Filter by name
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut self.service_picker_filter);
            });
            // List services filtered by:
            //   - target_type matching the selected object type
            //   - name contains filter text (case-insensitive)
            // Each row is clickable. On click:
            //   - Create AssignedService with the template ID
            //   - Add to the target (wall side/section, opening, or room)
            //   - Close the picker
        });
}
```

### Step 6.8 — Update compute_quantity ✅
**File:** `src/app.rs` (or wherever quantity calculation lives)

For wall services, `compute_quantity()` now takes the side/section context:
- `SquareMeter`: use `side.gross_area()` (or section area) minus opening areas
- `LinearMeter`: use `side.length` (or section length)
- `Piece`: 1

### Step 6.9 — Update Excel export ✅
**File:** `src/export/excel.rs`

In the "Estimate" sheet, wall services are grouped per-side:
```
Комната / Стена W1 (лев.) | Штукатурка | м² | 7.29 | 500 | 3645
Комната / Стена W1 (прав.) | Покраска | м² | 5.40 | 300 | 1620
```

### Verification
- Bottom panel is gone.
- "Услуги" button opens price list window. Can add/edit/delete services. Filter works.
- Selecting a wall shows per-side services in right panel.
- "Add service" opens picker. Filter works. Clicking assigns the service.
- Services show correct calculated quantity and cost.
- Excel export shows per-side services.
- Undo/redo works for service assignment changes.

---

## Phase 7: Wall Area Display (Change 7) ✅ DONE

### Goal
Show per-side wall area on the canvas. Each side shows gross and net (minus openings) area. Blue text on left, purple on right. Shown at all zoom levels.

### Step 7.1 — Compute opening deductions ✅
**File:** `src/app.rs` — `draw_walls()`

For each wall, compute total opening area:
```rust
let openings_area: f64 = wall.openings.iter()
    .filter_map(|oid| self.project.openings.iter().find(|o| o.id == *oid))
    .map(|o| o.kind.width() * o.kind.height())
    .sum();
```

### Step 7.2 — Render per-side area labels ✅
**File:** `src/app.rs` — `draw_walls()`, after the dimension label block (~line 2072-2091)

Add area labels on both sides of the wall:
```rust
let left_color = egui::Color32::from_rgb(100, 160, 220);   // blue
let right_color = egui::Color32::from_rgb(170, 100, 200);   // purple

let left_gross_m2 = wall.left_side.gross_area() / 1_000_000.0;
let left_net_m2 = (wall.left_side.gross_area() - openings_area) / 1_000_000.0;
let right_gross_m2 = wall.right_side.gross_area() / 1_000_000.0;
let right_net_m2 = (wall.right_side.gross_area() - openings_area) / 1_000_000.0;

// Left side label (positive normal direction)
let left_pos = egui::pos2(
    (start_screen.x + end_screen.x) / 2.0 + nx * 1.8,
    (start_screen.y + end_screen.y) / 2.0 + ny * 1.8,
);
let left_label = format!("S: {:.2} м² (чист: {:.2})", left_gross_m2, left_net_m2);
painter.text(left_pos, egui::Align2::CENTER_CENTER, left_label,
    egui::FontId::proportional(10.0), left_color);

// Right side label (negative normal direction)
let right_pos = egui::pos2(
    (start_screen.x + end_screen.x) / 2.0 - nx * 1.8,
    (start_screen.y + end_screen.y) / 2.0 - ny * 1.8,
);
let right_label = format!("S: {:.2} м² (чист: {:.2})", right_gross_m2, right_net_m2);
painter.text(right_pos, egui::Align2::CENTER_CENTER, right_label,
    egui::FontId::proportional(10.0), right_color);
```

### Verification
- Each wall shows blue area text on left, purple on right.
- Area values match side data.
- Net area accounts for openings.
- Labels visible at all zoom levels.

---

## Phase 8: T-Junction Support (Change 9) ✅ DONE

### Goal
Walls can attach to the side of another wall. The attached side splits into sections with independent properties and services. The other side is unaffected.

### Step 8.1 — Data model additions ✅
**File:** `src/model/wall.rs`

Add structs:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideJunction {
    pub wall_id: Uuid,  // connecting wall
    pub t: f64,         // parametric position (0.0 = start, 1.0 = end)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionData {
    pub length: f64,
    pub height_start: f64,
    pub height_end: f64,
}
```

Add fields to `SideData`:
```rust
pub struct SideData {
    pub length: f64,
    pub height_start: f64,
    pub height_end: f64,
    /// T-junctions on this side, sorted by t
    pub junctions: Vec<SideJunction>,
    /// Section properties. Empty = no junctions (use whole-side data).
    /// When junctions exist: junctions.len() + 1 entries.
    pub sections: Vec<SectionData>,
}
```

Update `SideData::new()` to initialize `junctions: Vec::new(), sections: Vec::new()`.

Add methods to `SideData`:
```rust
/// Insert a junction and recompute sections.
pub fn add_junction(&mut self, wall_id: Uuid, t: f64) { ... }

/// Remove a junction by connecting wall ID and merge sections.
pub fn remove_junction(&mut self, wall_id: Uuid) { ... }

/// Returns true if this side has sections (junctions exist).
pub fn has_sections(&self) -> bool { !self.junctions.is_empty() }

/// Get the number of sections (1 if no junctions, N+1 if N junctions).
pub fn section_count(&self) -> usize {
    if self.junctions.is_empty() { 1 } else { self.junctions.len() + 1 }
}
```

`add_junction` logic:
1. Insert junction sorted by t.
2. Recompute sections: for N junctions, create N+1 sections.
3. Section boundaries at t values: [0.0, t1, t2, ..., 1.0].
4. Each section length = (t_end - t_start) * self.length.
5. Each section height_start / height_end = linear interpolation of side heights at boundary t values.

`remove_junction` logic:
1. Find and remove the junction.
2. If no junctions remain, clear sections.
3. Otherwise, recompute sections as above.

### Step 8.2 — Snap system extension ✅
**File:** `src/editor/snap.rs`

Add to `SnapType`:
```rust
WallEdge {
    wall_id: Uuid,
    side: WallSide,
    t: f64,
},
```

Update `snap()` function — after vertex snap, before grid snap:
```rust
// Wall edge snap: check proximity to wall side edges
let mut closest_edge_dist = f64::MAX;
let mut closest_edge: Option<(Uuid, WallSide, f64, Point2D)> = None;

for wall in walls {
    let dx = wall.end.x - wall.start.x;
    let dy = wall.end.y - wall.start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0 { continue; }
    let half_t = wall.thickness / 2.0;
    // Left normal: (-dy/len, dx/len)
    let lnx = -dy / len * half_t;
    let lny = dx / len * half_t;

    for (side, sign) in [(WallSide::Left, 1.0), (WallSide::Right, -1.0)] {
        let edge_start = Point2D::new(wall.start.x + lnx * sign, wall.start.y + lny * sign);
        let edge_end = Point2D::new(wall.end.x + lnx * sign, wall.end.y + lny * sign);
        let (t, proj) = world_pos.project_onto_segment(edge_start, edge_end);
        // Only snap to interior of edge (not endpoints)
        if t > 0.01 && t < 0.99 {
            let dist = world_pos.distance_to(proj);
            if dist < snap_radius_world && dist < closest_edge_dist {
                closest_edge_dist = dist;
                closest_edge = Some((wall.id, side, t, proj));
            }
        }
    }
}

if let Some((wall_id, side, t, pos)) = closest_edge {
    return SnapResult {
        position: pos,
        snap_type: SnapType::WallEdge { wall_id, side, t },
    };
}
```

Note: `WallSide` is imported from `model::room`. May need to move it to `model::wall` or a shared location since it's now used more broadly.

### Step 8.3 — Wall creation with T-junction ✅
**File:** `src/editor/wall_tool.rs` and `src/app.rs` (where wall creation is handled)

When the snap result is `SnapType::WallEdge { wall_id, side, t }`:
1. Create the new wall as usual. Its endpoint is at the snapped position (on the side surface).
2. Find the target wall by `wall_id`.
3. Call `target_wall.left_side.add_junction(new_wall_id, t)` or `right_side` depending on `side`.
4. Initialize services for the new sections (empty).

### Step 8.4 — Junction cleanup on wall deletion ✅
**File:** `src/history.rs` — `RemoveWallCommand`

When deleting a wall:
1. Check all other walls for junctions referencing the deleted wall's ID.
2. For each found junction, call `side.remove_junction(deleted_wall_id)`.
3. Also check if the deleted wall itself had junctions — the connecting walls may need their endpoint handling updated.

### Step 8.5 — Room detection with junctions ✅
**File:** `src/editor/room_detection.rs` — `WallGraph::build()`

Currently, each wall creates 2 vertices (start, end) and 2 directed edges. With junctions:

1. For each wall, collect junction points from BOTH sides.
2. Sort all junction t values for the wall.
3. Compute intermediate vertex positions: `wall.start + (wall.end - wall.start) * t`.
4. The wall produces multiple graph edges: one per segment between consecutive vertices.
5. Each edge still references the wall ID (for room wall_ids) but represents a segment.

The connecting wall's endpoint matches one of these intermediate vertices (within merge epsilon).

The cycle-finding algorithm is unchanged — it just operates on the expanded graph.

### Step 8.6 — Section rendering ✅
**File:** `src/app.rs` — `draw_walls()`

When a wall is **selected** and a side has sections:
1. Draw thin lines perpendicular to the wall at each junction point (section boundaries).
2. Color each section with a unique color from a palette. Render as thin colored strips along the side edge.
3. Section colors are only visible when the wall is selected.

Section color palette (for selected wall only):
```rust
const SECTION_COLORS: &[(u8, u8, u8)] = &[
    (100, 180, 240),  // light blue
    (240, 160, 100),  // orange
    (100, 220, 140),  // light green
    (220, 120, 220),  // pink
    (240, 220, 100),  // light yellow
    (120, 220, 220),  // cyan
];
```

### Step 8.7 — Properties panel for sections ✅
**File:** `src/app.rs` — wall properties section

When a side has sections, expand the side in the properties panel:
```
■ Правая сторона (фиолетовая)
  ├── Секция 1 (● оранжевая)
  │   ├── Длина: 1500 мм
  │   ├── Высота начала: 2700 мм
  │   └── Высота конца: 2700 мм
  └── Секция 2 (● зелёная)
      ├── Длина: 2500 мм
      ├── Высота начала: 2700 мм
      └── Высота конца: 2700 мм
```

Section labels use the section color.

### Step 8.8 — Per-section services ✅
**File:** `src/app.rs` — services in right panel

When a side has sections, show per-section services:
```
───── Услуги ─────
■ Правая сторона (фиолетовая)
  Секция 1 (● оранжевая)
    Штукатурка | 4.05 м² | 500₽ | 2025₽  [×]
    [+ Добавить услугу]
  Секция 2 (● зелёная)
    Покраска | 6.75 м² | 300₽ | 2025₽  [×]
    [+ Добавить услугу]
```

Update `ServiceTarget::WallSide { wall_id, side, section_index }` — `section_index` identifies which section.

### Step 8.9 — Update undo/redo ✅
**File:** `src/history.rs`

`AddWallCommand` must store junction info if the wall was attached via T-junction:
```rust
pub struct AddWallCommand {
    wall: Wall,
    /// Junction created on another wall's side (if T-junction attachment)
    junction_target: Option<(Uuid, WallSide, f64)>, // (target_wall_id, side, t)
}
```

On undo: remove the junction from the target wall. On redo: re-add it.

Similarly for `RemoveWallCommand`: must restore junctions that were on the deleted wall.

### Verification ✅
- Drawing a wall that ends on another wall's side creates a T-junction.
- The target wall's side is split into sections with correct properties.
- The other side is unaffected.
- Room detection finds rooms through T-junctions.
- Deleting a T-junction wall merges sections back.
- Per-section services work (add, remove, calculate quantity).
- Section colors visible when wall selected.
- Undo/redo works for T-junction operations.

---

## Phase 9: Wall Joint Rendering (Change 10) ✅ DONE

### Goal
At shared wall endpoints: one side has no intersection, the other fills the gap. For 3+ walls: hub polygon.

### Step 9.1 — Wall connectivity analysis
**File:** `src/editor/wall_joints.rs` (new file)

```rust
use crate::model::{Point2D, Wall};

/// Computed joint vertices for one end of a wall.
pub struct JointVertices {
    pub left: egui::Pos2,   // adjusted left-side vertex
    pub right: egui::Pos2,  // adjusted right-side vertex
}

/// A hub polygon at a junction with 3+ walls.
pub struct HubPolygon {
    pub vertices: Vec<egui::Pos2>,
    pub fill: egui::Color32,
}

/// Compute adjusted vertices for all walls at their endpoints.
/// Returns: HashMap<(wall_id, is_end), JointVertices>
/// Plus any hub polygons to draw.
pub fn compute_joints(
    walls: &[Wall],
    canvas: &Canvas,
    center: egui::Pos2,
) -> (HashMap<(Uuid, bool), JointVertices>, Vec<HubPolygon>) { ... }
```

### Step 9.2 — Two-wall miter joint
**File:** `src/editor/wall_joints.rs`

When exactly 2 walls share an endpoint:
1. For each wall, compute the left and right edge lines at the shared endpoint.
2. Intersect: wall1.left with wall2.right (or determine which pair based on wall angles).
3. The intersection point becomes the miter vertex for both walls.
4. Clamp miter length: if the miter point is too far from the junction (> 3× wall thickness), use a bevel (flat cap) instead.

Implementation:
```rust
fn compute_two_wall_miter(
    wall_a: &Wall, a_is_end: bool,
    wall_b: &Wall, b_is_end: bool,
    junction: egui::Pos2,
    canvas: &Canvas, center: egui::Pos2,
) -> (JointVertices, JointVertices) { ... }
```

For each wall, determine the outgoing direction from the junction. Sort the two walls by angle. The wall edge on the "inner" side (between the walls) gets the miter point. The edge on the "outer" side gets the other miter point.

### Step 9.3 — Three+ wall hub polygon
**File:** `src/editor/wall_joints.rs`

When 3+ walls share an endpoint:
1. For each wall, compute the outgoing angle from the junction.
2. Sort walls by angle.
3. For each wall, compute left and right edge endpoints at the junction (simple perpendicular offset, no extension).
4. Walk walls in angular order. Between consecutive walls, collect the edge endpoints.
5. Form a polygon from all collected edge endpoints in angular order.
6. This polygon is the "hub" — fill it with neutral wall color.
7. Each wall's joint vertices are its two edge endpoints (trimmed to the hub boundary).

### Step 9.4 — Update draw_walls
**File:** `src/app.rs` — `draw_walls()`

Before drawing individual walls, call `compute_joints()`. Then for each wall:
1. Look up joint vertices for start and end endpoints.
2. If joint vertices exist, use them instead of the simple perpendicular offset corners.
3. The wall quad becomes: `[start_left, end_left, end_right, start_right]` using joint-adjusted vertices.

After drawing all walls, draw hub polygons.

### Step 9.5 — T-junction end trimming
**File:** `src/editor/wall_joints.rs`

When a wall connects to another wall's side via T-junction:
1. The connecting wall's endpoint is on the host wall's side surface.
2. The connecting wall's end face should be flush with the host wall's side edge.
3. Compute the host wall's edge direction at the junction point.
4. Trim the connecting wall's end vertices to align with this edge.

### Step 9.6 — Register module
**File:** `src/editor/mod.rs`

Add: `pub mod wall_joints;`

### Verification
- Two walls meeting at a corner: inner side has clean join, outer side fills the gap.
- Three walls meeting: hub polygon fills the center, no overlaps.
- T-junction wall end is flush with host wall surface.
- Very acute angles don't produce spike artifacts (bevel fallback).
- Wall side colors (blue/purple) are correct at joints.
- Works at all zoom levels.

---

## File Change Summary

| File | Phases |
|------|--------|
| `src/model/wall.rs` | 1, 8 |
| `src/model/opening.rs` | 3 |
| `src/model/project.rs` | 6 |
| `src/app.rs` | 1, 2, 3, 4, 5, 6, 7, 8, 9 |
| `src/editor/mod.rs` | 4, 9 |
| `src/editor/snap.rs` | 8 |
| `src/editor/wall_tool.rs` | 8 |
| `src/editor/room_detection.rs` | 5, 8 |
| `src/editor/triangulation.rs` (new) | 4 |
| `src/editor/wall_joints.rs` (new) | 9 |
| `src/export/excel.rs` | 1, 5, 6 |
| `src/history.rs` | 1, 8 |
