# Implementation Plan: Construction Estimate Application

Each phase from the technical specification is divided into discrete steps. Each step is a self-contained unit of work that an LLM can implement in a single session.

---

## Phase 1: Scaffold and Canvas (Basic Infrastructure)

### Step 1.1 — Project scaffold and dependencies ✅
- Initialize Rust project with `cargo init`.
- Configure `Cargo.toml` with dependencies: `eframe`, `egui`, `serde`, `serde_json`, `uuid`, `rfd`, `rust_xlsxwriter`.
- Create the module directory structure (`model/`, `editor/`, `panels/`, `export/`, `persistence/`).
- Create empty `mod.rs` files for each module.
- Verify the project compiles and runs an empty eframe window.

### Step 1.2 — Core data model: Point2D, Wall, Project ✅
- Define `Point2D` struct (x: f64, y: f64) with Serialize/Deserialize.
- Define `Wall` struct with fields: id (Uuid), start, end, thickness, height_start, height_end, openings (Vec<Uuid>).
- Define `Project` struct with fields: id, name, walls, openings, rooms, price_list_id, wall_services, opening_services, room_services.
- Implement Default values (thickness=200, heights=2700).
- Derive Serialize/Deserialize for all model structs.

### Step 1.3 — App struct and UI routing ✅
- Implement the main `App` struct holding `Project` and editor state.
- Set up `eframe::App` trait implementation with `update()`.
- Implement basic UI layout: top panel (toolbar placeholder), left panel (placeholder), central canvas area, right panel (placeholder), bottom panel (placeholder).
- Verify the app launches with the panel layout visible.

### Step 1.4 — Canvas: grid rendering ✅
- Implement `Canvas` struct with viewport state (offset, zoom).
- Render a background grid with configurable step size (default 100 mm).
- Implement adaptive grid density — show/hide sub-grid lines depending on zoom level.
- Display coordinate readout in a status bar (current cursor position in mm).

### Step 1.5 — Canvas: pan and zoom ✅
- Implement panning with middle mouse button drag.
- Implement panning with Space + LMB drag.
- Implement zoom with mouse wheel (zoom toward cursor position).
- Clamp zoom to reasonable min/max range.
- Implement world-to-screen and screen-to-world coordinate conversion methods.

### Step 1.6 — Snap system ✅
- Implement grid snapping: snap a world coordinate to the nearest grid point.
- Implement vertex snapping: snap to existing wall endpoints within 15px screen radius.
- Implement Shift modifier to disable snapping (free drawing).
- Return a `SnapResult` indicating the snap type and the snapped position.

### Step 1.7 — Project serialization/deserialization ✅
- Implement `project_io.rs`: save project to `saves/projects/{name}.json`.
- Implement load project from JSON file.
- Create `saves/projects/` and `saves/prices/` directories if they don't exist.
- Test round-trip: create a project with a wall, save, load, verify data integrity.

---

## Phase 2: Drawing Walls

### Step 2.1 — Wall drawing tool: basic two-click wall creation ✅
- Implement `WallTool` state machine: Idle → FirstPointSet → Drawing.
- On first click: record start point (with snap).
- On mouse move: show a preview line from start to current cursor (with snap).
- On second click: create a `Wall` and add it to the project.
- Return to Idle or continue chaining (see next step).

### Step 2.2 — Wall chaining and contour closing ✅
- After placing a wall, automatically start a new wall from the previous wall's endpoint.
- Double-click or Esc finishes the chain and returns to Idle.
- If the user clicks on the chain's starting vertex (within snap radius), close the contour and finish.

### Step 2.3 — Wall rendering on canvas ✅
- Render each wall as a thick line/rectangle on the canvas using the wall's thickness.
- Compute the perpendicular offset from the wall centerline to draw the rectangle.
- Use distinct colors for walls (e.g., dark gray fill, black outline).
- Render wall endpoints as small circles/dots.

### Step 2.4 — Tool selection and toolbar ✅
- Implement `EditorTool` enum: Select, Wall, Door, Window.
- Add toolbar buttons in the top panel for switching tools.
- Implement keyboard shortcuts: V (Select), W (Wall), D (Door), O (Window).
- Display the currently active tool in the toolbar.

### Step 2.5 — Selection tool: click selection and deletion ✅
- Implement `SelectTool`: on click, find the wall nearest to the cursor.
- Highlight the selected wall (different color/outline).
- On Delete key press, remove the selected wall from the project.
- Deselect on Escape or clicking empty space.

### Step 2.6 — Wall properties panel ✅
- When a wall is selected, show its properties in the right panel.
- Editable fields: thickness (mm), height_start (mm), height_end (mm).
- Read-only field: length (auto-calculated from start/end).
- Apply changes immediately to the wall on edit.

---

## Phase 3: Windows and Doors

### Step 3.1 — Opening data model ✅
- Define `OpeningKind` enum with `Door { height, width }` and `Window { height, width, sill_height, reveal_width }`.
- Define `Opening` struct with id, kind, wall_id (Option<Uuid>), offset_along_wall, assigned_services.
- Implement Default values (Door: 2100x900, Window: 1400x1200, sill=900, reveal=250).
- Derive Serialize/Deserialize.

### Step 3.2 — Opening placement tool: drag-and-drop onto walls ✅
- Implement `OpeningTool`: user selects Door or Window from toolbar.
- On click/drag on the canvas, detect if the cursor is over a wall.
- If over a wall: compute `offset_along_wall`, snap the opening to the wall, show preview.
- On release over a wall: create the Opening, attach it to the wall (set wall_id, add opening id to wall's openings).

### Step 3.3 — Opening rendering on canvas ✅
- Render doors on their parent wall as a gap/break in the wall with a swing arc indicator.
- Render windows on their parent wall as a gap with a parallel-line symbol.
- If an opening has `wall_id == None`, render it in red at its last known position.

### Step 3.4 — Opening dragging: along wall and between walls ✅
- When selecting an existing opening, allow dragging it along its wall (update offset_along_wall).
- If dragged off the wall and onto another wall, re-attach it (update wall_id, offset, update both walls' openings lists).
- If released outside any wall, set wall_id to None (validation error state).

### Step 3.5 — Opening validation ✅
- Check all openings: if any has `wall_id == None`, set a global validation error flag.
- Validate that the opening fits within the wall length (offset ± width/2 must be within 0..wall_length).
- Display validation errors in the properties panel.
- Disable the "Generate Report" button when validation errors exist.

### Step 3.6 — Opening properties panel ✅
- When an opening is selected, show its properties in the right panel.
- Door: height (mm), width (mm), depth (read-only, = wall thickness).
- Window: height (mm), width (mm), sill_height (mm), reveal_width (mm).
- Apply changes immediately.

---

## Phase 4: Room Detection

### Step 4.1 — Wall graph construction ✅
- Build a planar graph from walls: vertices are unique wall endpoints (merged within snap tolerance), edges are walls.
- Implement vertex merging: points within a small epsilon are considered the same vertex.
- Store adjacency data: for each vertex, the sorted list of connected edges with angles.

### Step 4.2 — Minimum cycle detection algorithm ✅
- Implement the minimum angle traversal algorithm to find all minimal enclosed faces.
- For each directed edge, find the next edge by choosing the smallest counter-clockwise turn.
- Collect all minimal cycles.
- Identify the outer boundary (largest cycle by area) and exclude it — it is not a room.

### Step 4.3 — Wall side determination ✅
- For each wall in a room's cycle, determine which side (left/right relative to the wall direction) faces the room interior.
- Define `WallSide` enum: Inner, Outer.
- Store `wall_sides` in the Room struct parallel to `wall_ids`.

### Step 4.4 — Floor area calculation ✅
- For each room, compute the inner polygon by offsetting walls inward by half-thickness on the room-facing side.
- Apply the Shoelace formula to compute the area of the inner polygon.
- Compute the perimeter as the sum of inner edge lengths.

### Step 4.5 — Room rendering on canvas ✅
- Render each detected room as a semi-transparent filled polygon with a unique color.
- Display the room name at the polygon's centroid.
- Use a palette of distinct colors for multiple rooms.

### Step 4.6 — Room list panel and renaming ✅
- Show the list of detected rooms in the left panel (or a dedicated section).
- Default room names: "Room 1", "Room 2", etc.
- Allow renaming rooms via an editable text field.
- Clicking a room in the list selects it on the canvas and shows properties in the right panel.
- Room properties: name, floor area (m²), perimeter (m).

### Step 4.7 — Auto-detection trigger ✅
- Hook the room detection algorithm to run whenever walls are added, removed, or modified.
- Preserve user-assigned room names when rooms are re-detected (match by wall set).
- Preserve assigned services when rooms persist through re-detection.

---

## Phase 5: Undo/Redo

### Step 5.1 — Command trait and History struct ✅
- Define the `Command` trait with `execute()`, `undo()`, and `description()` methods.
- Implement `History` struct with `undo_stack: Vec<Box<dyn Command>>` and `redo_stack: Vec<Box<dyn Command>>`.
- Implement `push()`, `undo()`, `redo()` methods on History.
- On push: clear redo_stack.

### Step 5.2 — Wall commands ✅
- Implement `AddWallCommand`: stores the wall data; execute adds it, undo removes it.
- Implement `RemoveWallCommand`: stores the wall data; execute removes it, undo re-adds it.
- Implement `ModifyWallCommand`: stores old and new wall data; execute applies new, undo restores old.

### Step 5.3 — Opening commands ✅
- Implement `AddOpeningCommand`, `RemoveOpeningCommand`, `ModifyOpeningCommand`.
- Handle wall references: when adding/removing an opening, also update the parent wall's openings list.

### Step 5.4 — Integrate commands into editor tools ✅
- Refactor wall tool to create walls via `AddWallCommand` pushed to History.
- Refactor selection tool deletion to use `RemoveWallCommand` / `RemoveOpeningCommand`.
- Refactor property edits to use `ModifyWallCommand` / `ModifyOpeningCommand`.
- Refactor opening tool to use `AddOpeningCommand`.

### Step 5.5 — Keyboard shortcuts for undo/redo ✅
- Bind Ctrl+Z to `history.undo()`.
- Bind Ctrl+Y and Ctrl+Shift+Z to `history.redo()`.
- Add Undo/Redo buttons to the toolbar with enabled/disabled state based on stack emptiness.

---

## Phase 6: Price List and Services

### Step 6.1 — Price list data model ✅
- Define `UnitType` enum: Piece, SquareMeter, LinearMeter.
- Define `TargetObjectType` enum: Wall, Window, Door, Room.
- Define `ServiceTemplate` struct: id, name, unit_type, price_per_unit, target_type.
- Define `PriceList` struct: name, services.
- Define `AssignedService` struct: service_template_id, custom_price (Option<f64>).
- Derive Serialize/Deserialize for all.

### Step 6.2 — Price list editing panel (bottom panel, "Price List" tab) ✅
- Render a table with columns: name, target object type, unit of measurement, price per unit.
- Add "Add Service" button: inserts a new row with default values.
- Add "Delete" button: removes the selected service.
- All fields are editable inline.

### Step 6.3 — Price list save/load ✅
- Implement `price_io.rs`: save price list to `saves/prices/{name}.json`.
- Implement load price list from JSON.
- Add "Import Price List" and "Export Price List" buttons to the price list panel.
- Use `rfd` for file dialogs.

### Step 6.4 — Service assignment to objects (bottom panel, "Assigned Services" tab) ✅
- When an object (wall/opening/room) is selected, show its assigned services.
- "Add Service" button: show a dropdown/popup of available services filtered by `target_type`.
- Display for each assigned service: name, calculated quantity, price per unit, total cost.
- Allow removing an assigned service.

### Step 6.5 — Quantity calculation for assigned services ✅
- Implement quantity calculation based on `unit_type`:
  - Piece → quantity = 1.
  - SquareMeter → wall: net area; room: floor area; window: reveal area; door: opening area.
  - LinearMeter → wall: length; room: perimeter; window: reveal perimeter; door: perimeter.
- Display calculated quantity and cost (quantity × price) in the services panel.

### Step 6.6 — Custom price override ✅
- Allow overriding the price per unit for a specific assigned service (custom_price field).
- If custom_price is set, use it instead of the template's price_per_unit.
- Display a visual indicator (e.g., bold or icon) when a custom price is active.

---

## Phase 7: Excel Report

### Step 7.1 — Excel export scaffold ✅
- Set up `rust_xlsxwriter` in `export/excel.rs`.
- Implement the function signature: `fn export_to_xlsx(project: &Project, price_list: &PriceList, path: &Path) -> Result<()>`.
- Create a workbook with 3 sheets: "Rooms", "Doors", "Estimate".
- Define cell formats: headers (bold), numbers (2 decimal places), currency.

### Step 7.2 — "Rooms" sheet: summary table ✅
- Write a summary table at the top: Room name, Floor Area (m²), Perimeter (m), Gross Wall Area (m²), Net Wall Area (m²).
- Iterate over all rooms and fill in the data.

### Step 7.3 — "Rooms" sheet: per-room detail breakdown ✅
- Below the summary, for each room write a detailed section.
- Walls sub-table: wall label, start height, end height, length, thickness, gross area, net area.
- Windows sub-table: label, height, width, reveal width, sill height, reveal perimeter, reveal area.
- Add spacing/headers between room sections.

### Step 7.4 — "Doors" sheet ✅
- Write the doors table: door label, height, width, depth, perimeter, from-room, to-room.
- Determine "from room" and "to room" by finding which rooms share the wall the door is on.

### Step 7.5 — "Estimate" sheet ✅
- Write the estimate table: Room/Object, Service name, Unit, Quantity, Price per unit, Cost.
- Group rows by room, then by object within the room.
- Add a TOTAL row at the bottom summing all costs.

### Step 7.6 — Report generation button and file dialog ✅
- Add "Generate Report" button to the toolbar.
- Disable the button when validation errors exist (e.g., unattached openings).
- On click, open a save file dialog (rfd) for the user to choose the output .xlsx path.
- Call `export_to_xlsx()` and show a success/error notification.

---

## Phase 8: Project Management

### Step 8.1 — Project list on startup ✅
- On app launch, scan `saves/projects/` for existing project JSON files.
- Display a project list/dialog: project name, last modified date.
- Options: Open selected, Create New, Delete selected.

### Step 8.2 — Create new project ✅
- "New Project" button/dialog: prompt for project name.
- Create a new empty `Project` with the given name.
- Save it to `saves/projects/{name}.json`.
- Switch the editor to the new project.
- Bind Ctrl+N shortcut.

### Step 8.3 — Open existing project ✅
- "Open" button: show file dialog (rfd) or project list.
- Load the selected project JSON.
- Switch the editor to the loaded project.
- Bind Ctrl+O shortcut.

### Step 8.4 — Save project ✅
- "Save" button: save the current project to its JSON file.
- Bind Ctrl+S shortcut.
- Show a brief confirmation (e.g., status bar message).

### Step 8.5 — Auto-save ✅
- Implement auto-save on every significant action: adding/removing/modifying walls, openings, services.
- Debounce if necessary to avoid excessive disk writes.
- Show an auto-save indicator in the status bar.

### Step 8.6 — Delete project ✅
- In the project list, allow deleting a project.
- Confirm with a dialog before deleting.
- Remove the JSON file from disk.

---

## Phase 9: Polish

### Step 9.1 — Left panel: project structure tree
- Implement a tree view in the left panel: Rooms → Walls → Openings.
- Each room node expands to show its walls; each wall node expands to show its openings.
- Clicking an item selects it on the canvas and shows its properties.

### Step 9.2 — Tooltips and UI hints
- Add tooltips to toolbar buttons (tool name + shortcut key).
- Add tooltips to panel controls explaining their function.
- Show a brief instruction text on the canvas when a tool is active (e.g., "Click to place wall start point").

### Step 9.3 — Keyboard shortcuts consolidation
- Verify all keyboard shortcuts from the spec are implemented: V, W, D, O, Delete, Ctrl+Z, Ctrl+Y, Ctrl+S, Ctrl+N, Ctrl+O, Escape, Shift.
- Add a help dialog or shortcut reference (accessible via F1 or Help menu).

### Step 9.4 — Performance optimization
- Profile rendering performance on large projects (many walls, rooms).
- Optimize canvas rendering: cull off-screen elements, cache computed geometry.
- Optimize room detection: avoid recomputation when unrelated changes occur.
- Test on low-end hardware and ensure smooth 60 FPS.

### Step 9.5 — Edge case handling
- Handle overlapping walls gracefully.
- Handle walls with zero length (prevent creation).
- Handle opening wider than wall length (prevent or clamp).
- Handle degenerate rooms (self-intersecting contours).
- Handle empty project (no walls) — disable report button, show helpful message.

### Step 9.6 — Russian language interface
- Ensure all UI text (labels, buttons, tooltips, dialogs) is in Russian.
- Verify correct encoding (UTF-8) in all user-facing strings.
- Verify Russian text renders correctly in egui (font support).
