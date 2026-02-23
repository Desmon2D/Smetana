# Architecture Overview

## System Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                         main.rs                                 │
│  eframe::run_native() → creates App, opens 1280x720 window     │
└────────────────────────────┬────────────────────────────────────┘
                             │ calls App::update() every frame
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      app/ (UI Layer)                            │
│                                                                 │
│  mod.rs ─── App struct, eframe::App impl, screen dispatch       │
│  ├── toolbar.rs ──────── top toolbar: tools, undo/redo, save    │
│  ├── canvas.rs ──────── central panel: input → tool dispatch    │
│  ├── canvas_draw.rs ──── two-pass wall/opening/room rendering   │
│  ├── project_list.rs ─── ProjectList screen (startup)           │
│  ├── properties_panel.rs  right panel: wall/opening/room props  │
│  ├── property_edits.rs ── validation helpers, section editors   │
│  ├── price_list.rs ────── price list CRUD window                │
│  ├── service_picker.rs ── service assignment picker dialog      │
│  └── services_panel.rs ── assigned services display + helpers   │
└──────┬──────────────┬──────────────┬───────────────┬────────────┘
       │              │              │               │
       ▼              ▼              ▼               ▼
┌────────────┐ ┌────────────┐ ┌───────────┐ ┌──────────────┐
│  editor/   │ │  model/    │ │ history.rs│ │ persistence/ │
│            │ │            │ │           │ │              │
│ Canvas     │ │ Wall       │ │ History   │ │ project_io   │
│ WallTool   │ │ Opening    │ │ (snapshot │ │ price_io     │
│ OpeningTool│ │ Room       │ │  undo/    │ │              │
│ Snap       │ │ Label      │ │  redo)    │ │ saves/       │
│ RoomDetect │ │ Project    │ │           │ │  projects/   │
│ RoomMetrics│ │ PriceList  │ │           │ │  prices/     │
│ WallJoints │ │ Quantity   │ │           │ │              │
│ Triangulate│ │            │ │           │ │              │
└────────────┘ └────────────┘ └───────────┘ └──────────────┘
                                                   │
                                                   ▼
                                           ┌──────────────┐
                                           │   export/    │
                                           │  excel.rs    │
                                           │  excel_sheets│
                                           │  (.xlsx)     │
                                           └──────────────┘
```

## Data Flow: User Input → State Mutation → Rendering

### Frame Lifecycle (`App::update`)

```
1. Screen dispatch
   ├── ProjectList → show_project_list()
   └── Editor:
       a. handle_keyboard_shortcuts() ← Ctrl+Z/Y/S/N/O, tool hotkeys V/W/D/O/T
       b. show_toolbar()              ← tool selection, undo/redo buttons, save/export
       c. show_left_panel()           ← project structure tree, room list
       d. show_right_panel()          ← selected object properties, assigned services
       e. show_price_list_window_ui() ← floating window for price list CRUD
       f. show_service_picker_window()← floating dialog for picking a service to assign
       g. show_project_settings_window() ← floating window for project defaults
       h. show_canvas()               ← THE MAIN LOOP (see below)
       i. auto_save()                 ← save project if version changed
```

### Canvas Input → State Mutation Pipeline

```
show_canvas():
  1. Handle pan (middle-drag or Space+primary-drag)
  2. Handle zoom (scroll wheel → zoom_toward)
  3. Update cursor_world_pos (screen_to_world)
  4. Tool-specific input:
     ├── Wall tool:
     │   ├── Update snap preview (snap() → preview_end, last_snap)
     │   ├── Double-click → reset tool
     │   ├── First click (Idle) → store chain_start, start_snap, chain_start_snap, transition to Drawing
     │   └── Second click (Drawing) → snapshot + project.add_wall(wall, junction_target, start_junction_target)
     │       ├── start_junction_target: computed from start_snap (T-junction at wall's start point)
     │       ├── junction_target: computed from last_snap (T-junction at wall's end point)
     │       ├── Check closing (snapped near chain_start) → close contour
     │       │   └── junction_target from chain_start_snap (T-junction back at chain origin)
     │       └── Otherwise → chain_from(snapped), start_snap = last_snap for next wall
     ├── Select tool:
     │   ├── Click → hit-test openings, then walls → set Selection
     │   ├── Drag opening → re-attach to wall under cursor
     │   ├── Escape → deselect
     │   └── Delete → snapshot + delete_selected()
     ├── Door/Window tool:
     │   ├── Hover → find wall under cursor → set hover_wall_id + hover_offset
     │   └── Click → snapshot + project.add_opening(opening)
     └── Label tool:
         └── Click → snapshot + project.labels.push(label)
  5. Room detection: WallGraph::build() (incl. T-junction vertex merge) → detect_rooms() → merge_rooms()
  6. Drawing:
     ├── draw_rooms()          ← triangulated fill + name/area labels
     ├── draw_walls()          ← pass 1: opaque section quads, joints, outline; pass 2: overlays
     ├── draw_openings()       ← gap cut + door arc / window parallel lines
     ├── draw_wall_preview()   ← preview line + snap indicator
     └── draw_opening_preview()← translucent placement preview
  7. Status bar (coordinates + zoom)
```

### History / Snapshot Pipeline

```
User action (click, drag, property edit)
  │
  ├── Canvas actions (undoable):
  │   history.snapshot(&project, "description")
  │   → project state cloned to undo_stack (VecDeque, 100-entry cap)
  │   → redo_stack cleared, version incremented
  │   → direct mutation via Project methods (add_wall, remove_wall, etc.)
  │
  └── Property panel edits (batched undo):
      → On first frame of editing: snapshot if edit_snapshot_version != history.version
      → DragValue mutations go directly to project fields (wall.thickness, etc.)
      → All changes accumulate into one undo step until selection changes

Undo: history.undo(project) → swap project with undo_stack.pop(), push current to redo_stack
Redo: history.redo(project) → swap project with redo_stack.pop(), push current to undo_stack
```

### Persistence Pipeline

```
auto_save() (every frame, if version changed)
  └── save_project(&project) → serialize to JSON → write saves/projects/{name}.json

Manual save: Ctrl+S or toolbar button → save_current_project()
Manual load: project list → open_project_from_path() → load_project() → deserialize + fixup

Price list: import/export via file dialogs (JSON format, saves/prices/{name}.json)
Excel export: toolbar button → rfd file dialog → export_to_xlsx() → 3-sheet workbook
```

### Coordinate System

```
World coordinates (mm) ──[Canvas.world_to_screen]──▶ Screen coordinates (px)
                        ◀──[Canvas.screen_to_world]──

zoom = pixels per mm (default 0.5, range 0.02–5.0)
offset = pan offset in world coordinates (mm)

Snap pipeline: raw cursor → screen_to_world → snap() → snapped world position
  Priority: vertex (15px radius) > wall edge > grid (100mm) > free (Shift held)
```
