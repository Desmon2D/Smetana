# State Management

## App Struct Fields — `src/app/mod.rs:32`

### Screen Navigation State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `screen` | `AppScreen` | Current screen (`ProjectList` or `Editor`) | `open_project_from_path`, `create_new_project`, toolbar "Открыть" |

### Project List Screen State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `project_entries` | `Vec<ProjectEntry>` | Cached list of saved projects | `refresh_project_list()` |
| `project_list_selection` | `Option<usize>` | Selected row in project list | User click, `refresh_project_list()` |
| `new_project_name` | `String` | Text input for new project name | User typing |
| `confirm_delete` | `Option<usize>` | Index of project pending delete confirmation | Delete button click |
| `show_new_project_dialog` | `bool` | Whether the "new project" dialog is open | Ctrl+N, toolbar button |

### Project Data State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `project` | `Project` | **Core project data** — walls, openings, rooms, services | Commands (via History), direct mutation in property panel, room merge |
| `price_list` | `PriceList` | Current price list (services catalog) | Price list window UI, import |

### Editor State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `editor` | `EditorState` | Active tool, selection, canvas viewport, tool states | Tool selection, canvas input, keyboard shortcuts |

`EditorState` contains:

| Sub-field | Type | Purpose |
|-----------|------|---------|
| `editor.active_tool` | `EditorTool` | Currently active tool (Select/Wall/Door/Window) |
| `editor.selection` | `Selection` | Currently selected object (None/Wall/Opening/Room) |
| `editor.canvas` | `Canvas` | Viewport: offset, zoom, grid_step, cursor_world_pos |
| `editor.wall_tool` | `WallTool` | Wall drawing state machine, chain tracking, snap state |
| `editor.opening_tool` | `OpeningTool` | Hover wall ID and offset for opening placement |

### History State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `history` | `History` | Undo/redo stacks with version counter | `push()`, `push_already_applied()`, `undo()`, `redo()` |
| `wall_edit_snapshot` | `Option<(Uuid, WallProps)>` | Snapshot of wall properties before panel editing | `update_edit_snapshots()`, `flush_property_edits()` |
| `opening_edit_snapshot` | `Option<(Uuid, OpeningKind)>` | Snapshot of opening kind before panel editing | `update_edit_snapshots()`, `flush_property_edits()` |

### UI State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `selected_service_idx` | `Option<usize>` | Selected row in price list window | User click |
| `status_message` | `Option<String>` | Status bar message (save confirmations, errors) | Save/load/export operations |
| `show_price_list_window` | `bool` | Price list window visibility | "Услуги" button toggle |
| `show_service_picker` | `bool` | Service picker dialog visibility | "+ Добавить услугу" button |
| `service_picker_filter` | `String` | Filter text in service picker | User typing |
| `service_picker_target` | `Option<ServiceTarget>` | Target object for service assignment | Set when picker opens |
| `price_list_filter` | `String` | Filter text in price list window | User typing |
| `label_scale` | `f32` | Canvas label font size multiplier (0.5–3.0, default 1.0) | Left panel slider |

### Persistence Tracking

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `last_saved_version` | `u64` | History version at last save | `save_current_project()`, `auto_save()` |
| `dirty` | `bool` | True if non-history changes need saving (room name edits, service assignments) | Direct mutations, `auto_save()` |

## State Mutation Patterns

### Pattern 1: Command-Based Mutation (Undoable)

Used for wall/opening creation, deletion, and property changes.

```
1. User action (click, Delete key, DragValue commit)
2. flush_property_edits() — commit pending panel edits first
3. history.push(Box::new(SomeCommand { ... }), &mut project)
   → cmd.execute() modifies project
   → cmd stored in undo_stack
   → redo_stack cleared
   → version++
4. auto_save() detects version change → saves
```

### Pattern 2: Deferred Edit Snapshots (Property Panel)

Used for wall thickness/height/length and opening dimension edits via DragValue.

```
1. User selects wall/opening → snapshot captured (wall_edit_snapshot / opening_edit_snapshot)
2. User drags DragValue → direct mutation of project.walls[i].thickness etc.
3. On selection change or before next command: flush_property_edits()
   → Compare current values vs snapshot
   → If changed: push ModifyWallCommand/ModifyOpeningCommand (already-applied)
   → Snapshot cleared
```

### Pattern 3: Direct Mutation (Non-Undoable, Dirty Flag)

Used for room names, service assignments, service picker.

```
1. User edits room name or assigns/removes service
2. Direct mutation of project fields
3. self.dirty = true
4. auto_save() checks dirty flag → saves
```

### Pattern 4: Room Detection (Computed, Every Frame)

```
1. show_canvas() runs every frame
2. WallGraph::build(&project.walls) → detect_rooms() → new_rooms
3. merge_rooms(new_rooms):
   → Match new rooms to existing rooms by sorted wall_ids
   → Preserve id, name, and services of matched rooms
   → Remove services of disappeared rooms
   → Replace project.rooms with merged result
```

## State Read Patterns During Rendering

### Canvas Drawing (read-only traversal)

```
draw_walls():   reads project.walls, editor.canvas, editor.selection, label_scale
draw_openings(): reads project.openings, project.walls, editor.canvas, editor.selection, label_scale
draw_rooms():   reads project.rooms, project.walls (via compute_room_metrics), label_scale
```

### Properties Panel (mixed read/write)

```
show_right_panel():
  match editor.selection:
    Wall(id)    → mutable borrow of project.walls[id] for DragValue editors
    Opening(id) → mutable borrow of project.openings[id] for DragValue editors
    Room(id)    → mutable borrow of project.rooms[id] for name TextEdit
                  read-only compute_room_metrics for display
```

### Services Panel (read project, write services)

```
show_wall_side_services():
  reads project.walls (for quantity computation)
  reads project.wall_services (for display)
  writes project.wall_services (add/remove/price change)

show_flat_services():
  reads price_list (for template lookup)
  writes project.opening_services or project.room_services
```

## Key State Invariants

1. **`wall.openings` mirrors `opening.wall_id`**: Every opening with `wall_id = Some(wid)` must have its `id` in `walls[wid].openings`, and vice versa. Commands maintain this bidirectional link.

2. **Sections match junctions**: `side.sections.len() == side.junctions.len() + 1` after any junction add/remove. Ensured by `recompute_sections()`.

3. **Room wall_ids/wall_sides are parallel**: `room.wall_ids[i]` corresponds to `room.wall_sides[i]`.

4. **History version monotonically increases**: `version` increments on every push, undo, or redo. Used to detect unsaved changes via `last_saved_version`.

5. **Edit snapshots are per-selection**: At most one wall snapshot and one opening snapshot exist at a time. They are flushed when selection changes or before any history command.
