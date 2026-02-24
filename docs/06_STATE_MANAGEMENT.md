# State Management

## App Struct Fields — `src/app/mod.rs`

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
| `new_project_defaults` | `ProjectDefaults` | Temporary defaults for new project dialog | User editing in dialog |
| `show_project_settings` | `bool` | Whether the project settings window is open | "Настройки" toolbar button |

### Project Data State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `project` | `Project` | **Core project data** — walls, openings, rooms, labels, services | Snapshot undo/redo swaps entire Project; direct mutation via Project methods and property panel |
| `price_list` | `PriceList` | Current price list (services catalog) | Price list window UI, import |

### Editor State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `editor` | `EditorState` | Active tool, selection, canvas viewport, tool states, orphan positions | Tool selection, canvas input, keyboard shortcuts |

`EditorState` contains:

| Sub-field | Type | Purpose |
|-----------|------|---------|
| `editor.active_tool` | `EditorTool` | Currently active tool (Select/Wall/Door/Window/Label) |
| `editor.selection` | `Selection` | Currently selected object (None/Wall/Opening/Room/Label) |
| `editor.canvas` | `Canvas` | Viewport: offset, zoom, grid_step, cursor_world_pos |
| `editor.wall_tool` | `WallTool` | Wall drawing state machine, chain tracking, snap state |
| `editor.opening_tool` | `OpeningTool` | Hover wall ID and offset for opening placement |
| `editor.orphan_positions` | `HashMap<Uuid, DVec2>` | Transient world positions for detached openings (not serialized) |

### History State

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `history` | `History` | Snapshot-based undo/redo: `VecDeque<(Project, &str)>` stacks, version counter, 100-entry cap | `snapshot()`, `undo()`, `redo()`, `mark_dirty()` |
| `edit_snapshot_version` | `Option<u64>` | History version when property editing snapshot was taken; ensures DragValue changes accumulate into one undo step | Set in `show_right_panel()`, cleared on selection change / undo / redo |

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
| `rooms_version` | `u64` | History version at last room detection run; gates re-detection | `show_canvas()`, reset on project load/create |

### Persistence Tracking

| Field | Type | Purpose | Mutated By |
|-------|------|---------|------------|
| `last_saved_version` | `u64` | History version at last save | `save_current_project()`, `auto_save()` |

## State Mutation Patterns

### Pattern 1: Snapshot-Based Mutation (Undoable)

Used for wall/opening/label creation, deletion, and canvas actions.

```
1. User action (click, Delete key)
2. history.snapshot(&project, "description")
   → project cloned to undo_stack (VecDeque, 100-entry cap)
   → redo_stack cleared, version incremented
3. Direct mutation via Project methods:
   → project.add_wall(wall, junction_target, start_junction_target)
   → project.remove_wall(id) / project.remove_opening(id) / project.remove_label(id)
   → project.add_opening(opening)
   → project.labels.push(label)
4. auto_save() detects version change → saves
```

### Pattern 2: Batched Property Edits (Single Undo Step)

Used for wall thickness/height/length and opening dimension edits via DragValue.

```
1. User selects wall/opening/label → show_right_panel()
2. If edit_snapshot_version != history.version:
   → history.snapshot(&project, "edit properties")
   → edit_snapshot_version = Some(history.version)
3. User drags DragValue → direct mutation of project fields (wall.thickness, etc.)
4. All changes accumulate under one snapshot until selection changes or undo/redo resets
```

### Pattern 3: Direct Mutation (Non-Undoable, Version Bump)

Used for room names, service assignments, service picker.

```
1. User edits room name or assigns/removes service
2. Direct mutation of project fields
3. self.history.mark_dirty() → bumps version without snapshot
4. auto_save() detects version change → saves
```

### Pattern 4: Room Detection (Computed, Version-Gated)

```
1. show_canvas() checks: history.version != rooms_version
2. If changed: WallGraph::build(&project.walls) → detect_rooms() → new_rooms
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
draw_openings(): reads project.openings, project.walls, editor.canvas, editor.selection, editor.orphan_positions, label_scale
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
    Label(id)   → mutable borrow of project.labels[id] for text/font/rotation editors
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

1. **`wall.openings` mirrors `opening.wall_id`**: Every opening with `wall_id = Some(wid)` must have its `id` in `walls[wid].openings`, and vice versa. `Project::add_opening`/`remove_opening`/`remove_wall` maintain this bidirectional link.

2. **Sections match junctions**: `side.sections.len() == side.junctions.len() + 1` after any junction add/remove. Ensured by `recompute_sections()`.

3. **Room wall_ids/wall_sides are parallel**: `room.wall_ids[i]` corresponds to `room.wall_sides[i]`.

4. **History version monotonically increases**: `version` increments on every snapshot, undo, redo, or mark_dirty. Used to detect unsaved changes via `last_saved_version`.

5. **Edit snapshot is per-version**: `edit_snapshot_version` ensures at most one snapshot per editing session. Reset on selection change, undo, or redo.
