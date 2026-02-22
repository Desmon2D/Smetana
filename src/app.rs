use eframe::egui;

use crate::editor::{EditorState, EditorTool, Selection, WallToolState, snap};
use crate::editor::room_detection::{WallGraph, compute_room_metrics};
use crate::history::{
    AddOpeningCommand, AddWallCommand, History, ModifyOpeningCommand, ModifyWallCommand,
    RemoveOpeningCommand, RemoveWallCommand,
};
use crate::model::{
    AssignedService, Opening, OpeningKind, Point2D, PriceList, Project, ServiceTemplate,
    TargetObjectType, UnitType, Wall,
};
use crate::export::export_to_xlsx;
use crate::persistence::{
    delete_project, list_project_entries, load_price_list, load_project, save_price_list_to,
    save_project, ProjectEntry,
};

enum ProjectListAction {
    Open(usize),
}

/// Format a SystemTime as "DD.MM.YYYY HH:MM".
fn format_system_time(t: std::time::SystemTime) -> String {
    match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => {
            let secs = dur.as_secs();
            // Simple UTC breakdown (good enough for file dates)
            let days = secs / 86400;
            let time_of_day = secs % 86400;
            let hours = time_of_day / 3600;
            let minutes = (time_of_day % 3600) / 60;

            // Days since 1970-01-01
            let (year, month, day) = days_to_ymd(days);
            format!("{day:02}.{month:02}.{year} {hours:02}:{minutes:02}")
        }
        Err(_) => "—".to_string(),
    }
}

/// Convert days since 1970-01-01 to (year, month, day).
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// Top-level app screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppScreen {
    /// Project list shown on startup.
    ProjectList,
    /// The main editor view.
    Editor,
}

/// Which tab is active in the bottom panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    PriceList,
    AssignedServices,
}

/// Display data for one row of the assigned-services table.
struct AssignedServiceRow {
    name: String,
    unit_label: String,
    /// Price from the template (before override).
    template_price: f64,
    /// Effective price (custom if set, otherwise template).
    effective_price: f64,
    /// Whether this row has a custom price override.
    has_custom: bool,
    qty: f64,
    /// True if the template was found in the price list.
    valid: bool,
}

pub struct App {
    /// Current top-level screen.
    screen: AppScreen,
    /// Cached project list entries (refreshed when entering ProjectList screen).
    project_entries: Vec<ProjectEntry>,
    /// Selected index in the project list.
    project_list_selection: Option<usize>,
    /// Name input for creating a new project.
    new_project_name: String,
    /// Index of project pending delete confirmation (None = no dialog).
    confirm_delete: Option<usize>,
    /// Whether the "New Project" dialog is open (in editor mode).
    show_new_project_dialog: bool,

    pub project: Project,
    pub editor: EditorState,
    pub history: History,
    /// Snapshot of wall properties at time of selection (for undo of property edits).
    wall_edit_snapshot: Option<(uuid::Uuid, f64, f64, f64)>,
    /// Snapshot of opening kind at time of selection (for undo of property edits).
    opening_edit_snapshot: Option<(uuid::Uuid, OpeningKind)>,
    /// The active price list.
    pub price_list: PriceList,
    /// Index of the selected service row in the price list (for deletion).
    selected_service_idx: Option<usize>,
    /// Transient status message shown in the bottom panel.
    status_message: Option<String>,
    /// Active tab in the bottom panel.
    bottom_tab: BottomTab,
    /// History version at last save (for auto-save detection).
    last_saved_version: u64,
    /// Set to true when non-history mutations occur (e.g. service assignment changes).
    dirty: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let project_entries = list_project_entries().unwrap_or_default();
        Self {
            screen: AppScreen::ProjectList,
            project_entries,
            project_list_selection: None,
            new_project_name: String::new(),
            confirm_delete: None,
            show_new_project_dialog: false,

            project: Project::new("Новый проект".to_string()),
            editor: EditorState::default(),
            history: History::new(),
            wall_edit_snapshot: None,
            opening_edit_snapshot: None,
            price_list: PriceList::new("Прайс-лист".to_string()),
            selected_service_idx: None,
            status_message: None,
            bottom_tab: BottomTab::PriceList,
            last_saved_version: 0,
            dirty: false,
        }
    }

    /// Refresh the cached project list from disk.
    fn refresh_project_list(&mut self) {
        self.project_entries = list_project_entries().unwrap_or_default();
        self.project_list_selection = None;
        self.confirm_delete = None;
    }

    /// Open a project from disk and switch to the editor.
    fn open_project_from_path(&mut self, path: &std::path::Path) {
        match load_project(path) {
            Ok(project) => {
                self.project = project;
                self.editor = EditorState::default();
                self.history = History::new();
                self.wall_edit_snapshot = None;
                self.opening_edit_snapshot = None;
                self.status_message = None;
                self.last_saved_version = 0;
                self.dirty = false;
                self.screen = AppScreen::Editor;
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка: {e}"));
            }
        }
    }

    /// Save the current project to disk and show a status message.
    fn save_current_project(&mut self) {
        match save_project(&self.project) {
            Ok(path) => {
                self.last_saved_version = self.history.version;
                self.dirty = false;
                self.status_message =
                    Some(format!("Проект сохранён: {}", path.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка сохранения: {e}"));
            }
        }
    }

    /// Auto-save if the project has unsaved changes (history version changed or dirty flag set).
    fn auto_save(&mut self) {
        if self.history.version != self.last_saved_version || self.dirty {
            if save_project(&self.project).is_ok() {
                self.last_saved_version = self.history.version;
                self.dirty = false;
            }
        }
    }

    /// Create a new empty project and switch to the editor.
    fn create_new_project(&mut self, name: String) {
        let project = Project::new(name);
        // Save immediately so it appears in the project list
        let _ = save_project(&project);
        self.project = project;
        self.editor = EditorState::default();
        self.history = History::new();
        self.wall_edit_snapshot = None;
        self.opening_edit_snapshot = None;
        self.status_message = None;
        self.last_saved_version = 0;
        self.dirty = false;
        self.screen = AppScreen::Editor;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.screen {
            AppScreen::ProjectList => self.show_project_list(ctx),
            AppScreen::Editor => {
                self.update_edit_snapshots();
                self.handle_keyboard_shortcuts(ctx);
                self.show_toolbar(ctx);
                self.show_left_panel(ctx);
                self.show_right_panel(ctx);
                self.show_bottom_panel(ctx);
                self.show_canvas(ctx);
                self.auto_save();
            }
        }
    }
}

impl App {
    fn show_project_list(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.heading("Сметана — Строительная смета");
                ui.add_space(20.0);
            });

            // --- Create new project ---
            ui.horizontal(|ui| {
                ui.label("Новый проект:");
                ui.text_edit_singleline(&mut self.new_project_name);
                let name_ok = !self.new_project_name.trim().is_empty();
                if ui
                    .add_enabled(name_ok, egui::Button::new("Создать"))
                    .clicked()
                {
                    let name = self.new_project_name.trim().to_string();
                    self.new_project_name.clear();
                    self.create_new_project(name);
                    return;
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            if self.project_entries.is_empty() {
                ui.label("Нет сохранённых проектов.");
                return;
            }

            ui.label("Сохранённые проекты:");
            ui.add_space(4.0);

            // --- Project table ---
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("project_list_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        // Header
                        ui.strong("Название");
                        ui.strong("Изменён");
                        ui.strong(""); // actions column
                        ui.end_row();

                        let mut action: Option<ProjectListAction> = None;

                        for (i, entry) in self.project_entries.iter().enumerate() {
                            let is_selected = self.project_list_selection == Some(i);

                            // Name (clickable)
                            if ui
                                .selectable_label(is_selected, &entry.name)
                                .clicked()
                            {
                                self.project_list_selection = Some(i);
                            }

                            // Modified date
                            let date_str = format_system_time(entry.modified);
                            ui.label(&date_str);

                            // Action buttons
                            ui.horizontal(|ui| {
                                if ui.button("Открыть").clicked() {
                                    action = Some(ProjectListAction::Open(i));
                                }
                                if ui.button("Удалить").clicked() {
                                    self.confirm_delete = Some(i);
                                }
                            });

                            ui.end_row();
                        }

                        if let Some(a) = action {
                            match a {
                                ProjectListAction::Open(i) => {
                                    let path = self.project_entries[i].path.clone();
                                    self.open_project_from_path(&path);
                                }
                            }
                        }
                    });
            });

            // Double-click selected entry to open
            if let Some(sel) = self.project_list_selection {
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let path = self.project_entries[sel].path.clone();
                    self.open_project_from_path(&path);
                }
            }

            // --- Delete confirmation dialog ---
            if let Some(del_idx) = self.confirm_delete {
                let name = self.project_entries[del_idx].name.clone();
                let mut open = true;
                egui::Window::new("Подтверждение")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut open)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(format!("Удалить проект «{name}»?"));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Удалить").clicked() {
                                let path = self.project_entries[del_idx].path.clone();
                                let _ = delete_project(&path);
                                self.refresh_project_list();
                            }
                            if ui.button("Отмена").clicked() {
                                self.confirm_delete = None;
                            }
                        });
                    });
                if !open {
                    self.confirm_delete = None;
                }
            }

            // Status message
            if let Some(msg) = &self.status_message {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::RED, msg);
            }
        });
    }

    /// Flush pending property edits if the selection no longer matches the snapshot.
    fn update_edit_snapshots(&mut self) {
        let wall_snap_matches = match (&self.wall_edit_snapshot, self.editor.selection) {
            (Some((snap_id, ..)), Selection::Wall(sel_id)) => *snap_id == sel_id,
            (None, _) => true,
            _ => false,
        };
        if !wall_snap_matches {
            self.flush_property_edits();
        }

        let opening_snap_matches = match (&self.opening_edit_snapshot, self.editor.selection) {
            (Some((snap_id, _)), Selection::Opening(sel_id)) => *snap_id == sel_id,
            (None, _) => true,
            _ => false,
        };
        if !opening_snap_matches {
            self.flush_property_edits();
        }
    }

    /// Commit any pending property edits to the history (unconditionally).
    fn flush_property_edits(&mut self) {
        if let Some((snap_id, old_t, old_hs, old_he)) = self.wall_edit_snapshot.take() {
            if let Some(wall) = self.project.walls.iter().find(|w| w.id == snap_id) {
                if (wall.thickness - old_t).abs() > 0.01
                    || (wall.height_start - old_hs).abs() > 0.01
                    || (wall.height_end - old_he).abs() > 0.01
                {
                    self.history.push_already_applied(Box::new(
                        ModifyWallCommand::from_values(
                            snap_id,
                            old_t, old_hs, old_he,
                            wall.thickness, wall.height_start, wall.height_end,
                        ),
                    ));
                }
            }
        }
        if let Some((snap_id, old_kind)) = self.opening_edit_snapshot.take() {
            if let Some(opening) = self.project.openings.iter().find(|o| o.id == snap_id) {
                if opening_kind_changed(&old_kind, &opening.kind) {
                    self.history.push_already_applied(Box::new(
                        ModifyOpeningCommand::from_values(snap_id, old_kind, opening.kind.clone()),
                    ));
                }
            }
        }
    }
}

fn opening_kind_changed(a: &OpeningKind, b: &OpeningKind) -> bool {
    match (a, b) {
        (
            OpeningKind::Door { height: h1, width: w1 },
            OpeningKind::Door { height: h2, width: w2 },
        ) => (h1 - h2).abs() > 0.01 || (w1 - w2).abs() > 0.01,
        (
            OpeningKind::Window { height: h1, width: w1, sill_height: s1, reveal_width: r1 },
            OpeningKind::Window { height: h2, width: w2, sill_height: s2, reveal_width: r2 },
        ) => {
            (h1 - h2).abs() > 0.01 || (w1 - w2).abs() > 0.01
                || (s1 - s2).abs() > 0.01 || (r1 - r2).abs() > 0.01
        }
        _ => true,
    }
}

impl App {
    /// Returns true if any opening has a validation error.
    fn has_validation_errors(&self) -> bool {
        for opening in &self.project.openings {
            if opening.wall_id.is_none() {
                return true;
            }
            if let Some(wid) = opening.wall_id {
                match self.project.walls.iter().find(|w| w.id == wid) {
                    None => return true,
                    Some(wall) => {
                        let wall_len = wall.length();
                        let half_w = opening.kind.width() / 2.0;
                        if opening.offset_along_wall - half_w < -1.0
                            || opening.offset_along_wall + half_w > wall_len + 1.0
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Collect validation error messages for a specific opening.
    fn opening_errors(&self, opening: &Opening) -> Vec<&'static str> {
        let mut errors = Vec::new();
        match opening.wall_id {
            None => {
                errors.push("Проём не привязан к стене");
            }
            Some(wid) => match self.project.walls.iter().find(|w| w.id == wid) {
                None => {
                    errors.push("Стена не найдена");
                }
                Some(wall) => {
                    let wall_len = wall.length();
                    let half_w = opening.kind.width() / 2.0;
                    if opening.offset_along_wall - half_w < -1.0
                        || opening.offset_along_wall + half_w > wall_len + 1.0
                    {
                        errors.push("Проём выходит за пределы стены");
                    }
                }
            },
        }
        errors
    }

    /// Merge newly detected rooms with existing rooms, preserving user data.
    ///
    /// Rooms are matched by their wall set (sorted wall IDs). If a new room
    /// matches an existing one, the old room's id, name, and services are kept.
    /// Rooms that no longer exist have their services cleaned up.
    fn merge_rooms(&mut self, new_rooms: Vec<crate::model::Room>) {
        use std::collections::HashMap;

        // Build a lookup from sorted wall set → old room index
        let mut old_by_walls: HashMap<Vec<uuid::Uuid>, usize> = HashMap::new();
        for (i, room) in self.project.rooms.iter().enumerate() {
            let mut key: Vec<uuid::Uuid> = room.wall_ids.clone();
            key.sort();
            old_by_walls.insert(key, i);
        }

        let old_rooms = std::mem::take(&mut self.project.rooms);
        let mut merged = Vec::with_capacity(new_rooms.len());
        let mut preserved_ids: Vec<uuid::Uuid> = Vec::new();

        for mut new_room in new_rooms {
            let mut key: Vec<uuid::Uuid> = new_room.wall_ids.clone();
            key.sort();

            if let Some(&old_idx) = old_by_walls.get(&key) {
                let old = &old_rooms[old_idx];
                // Preserve id, name, and keep new geometry (wall_sides may have changed)
                new_room.id = old.id;
                new_room.name = old.name.clone();
                preserved_ids.push(old.id);
            }

            merged.push(new_room);
        }

        // Clean up services for rooms that no longer exist
        let preserved_set: std::collections::HashSet<uuid::Uuid> =
            preserved_ids.into_iter().collect();
        let old_ids: Vec<uuid::Uuid> = old_rooms.iter().map(|r| r.id).collect();
        for old_id in old_ids {
            if !preserved_set.contains(&old_id) {
                self.project.room_services.remove(&old_id);
            }
        }

        self.project.rooms = merged;
    }

    /// Switch to a new tool, resetting state of the previous tool.
    fn set_tool(&mut self, tool: EditorTool) {
        if self.editor.active_tool != tool {
            // Reset wall tool state when leaving Wall mode
            if self.editor.active_tool == EditorTool::Wall {
                self.editor.wall_tool.reset();
            }
            self.editor.active_tool = tool;
        }
    }

    /// Handle global keyboard shortcuts for tool switching and undo/redo.
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let (ctrl_z, ctrl_y, ctrl_shift_z, ctrl_n, ctrl_o, ctrl_s) = ctx.input(|i| {
            (
                i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift,
                i.modifiers.ctrl && i.key_pressed(egui::Key::Y),
                i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z),
                i.modifiers.ctrl && i.key_pressed(egui::Key::N),
                i.modifiers.ctrl && i.key_pressed(egui::Key::O),
                i.modifiers.ctrl && i.key_pressed(egui::Key::S),
            )
        });

        if ctrl_s {
            self.save_current_project();
        } else if ctrl_n {
            self.show_new_project_dialog = true;
        } else if ctrl_o {
            self.refresh_project_list();
            self.screen = AppScreen::ProjectList;
        } else if ctrl_z {
            self.flush_property_edits();
            self.history.undo(&mut self.project);
        } else if ctrl_y || ctrl_shift_z {
            self.flush_property_edits();
            self.history.redo(&mut self.project);
        }

        ctx.input(|i| {
            // Tool shortcuts (only when no modifier keys are held)
            if !i.modifiers.ctrl && !i.modifiers.alt {
                if i.key_pressed(egui::Key::V) {
                    self.set_tool(EditorTool::Select);
                } else if i.key_pressed(egui::Key::W) {
                    self.set_tool(EditorTool::Wall);
                } else if i.key_pressed(egui::Key::D) {
                    self.set_tool(EditorTool::Door);
                } else if i.key_pressed(egui::Key::O) {
                    self.set_tool(EditorTool::Window);
                }
            }
        });
    }

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Инструмент:");

                let prev_tool = self.editor.active_tool;
                let tool = &mut self.editor.active_tool;
                ui.selectable_value(tool, EditorTool::Select, "Выбор (V)");
                ui.selectable_value(tool, EditorTool::Wall, "Стена (W)");
                ui.selectable_value(tool, EditorTool::Door, "Дверь (D)");
                ui.selectable_value(tool, EditorTool::Window, "Окно (O)");

                // Reset wall tool if user clicked a different tool button
                if prev_tool == EditorTool::Wall && self.editor.active_tool != EditorTool::Wall {
                    self.editor.wall_tool.reset();
                }

                ui.separator();

                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("Отменить"))
                    .clicked()
                {
                    self.flush_property_edits();
                    self.history.undo(&mut self.project);
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("Повторить"))
                    .clicked()
                {
                    self.flush_property_edits();
                    self.history.redo(&mut self.project);
                }

                ui.separator();

                if ui.button("Новый проект").clicked() {
                    self.show_new_project_dialog = true;
                }
                if ui.button("Открыть").clicked() {
                    self.refresh_project_list();
                    self.screen = AppScreen::ProjectList;
                }
                if ui.button("Сохранить").clicked() {
                    self.save_current_project();
                }

                let can_report = !self.has_validation_errors();
                if ui
                    .add_enabled(can_report, egui::Button::new("Сформировать отчёт"))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Сохранить отчёт")
                        .add_filter("Excel", &["xlsx"])
                        .set_file_name(&format!("{}.xlsx", self.project.name))
                        .save_file()
                    {
                        match export_to_xlsx(&self.project, &self.price_list, &path) {
                            Ok(()) => {
                                self.status_message =
                                    Some(format!("Отчёт сохранён: {}", path.display()));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Ошибка: {e}"));
                            }
                        }
                    }
                }
            });
        });

        // New project dialog (modal window)
        if self.show_new_project_dialog {
            let mut open = true;
            egui::Window::new("Новый проект")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Название:");
                        ui.text_edit_singleline(&mut self.new_project_name);
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let name_ok = !self.new_project_name.trim().is_empty();
                        if ui.add_enabled(name_ok, egui::Button::new("Создать")).clicked() {
                            let name = self.new_project_name.trim().to_string();
                            self.new_project_name.clear();
                            self.show_new_project_dialog = false;
                            self.create_new_project(name);
                        }
                        if ui.button("Отмена").clicked() {
                            self.new_project_name.clear();
                            self.show_new_project_dialog = false;
                        }
                    });
                });
            if !open {
                self.new_project_name.clear();
                self.show_new_project_dialog = false;
            }
        }
    }

    fn show_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Структура проекта");
                ui.separator();
                ui.label(format!("Стен: {}", self.project.walls.len()));
                ui.label(format!("Проёмов: {}", self.project.openings.len()));

                ui.add_space(8.0);
                ui.separator();
                ui.label(format!("Комнаты ({})", self.project.rooms.len()));
                ui.add_space(4.0);

                let selected_room = match self.editor.selection {
                    Selection::Room(id) => Some(id),
                    _ => None,
                };

                let mut clicked_room = None;
                for room in &self.project.rooms {
                    let is_selected = selected_room == Some(room.id);
                    let label = egui::SelectableLabel::new(is_selected, &room.name);
                    if ui.add(label).clicked() {
                        clicked_room = Some(room.id);
                    }
                }

                if let Some(id) = clicked_room {
                    self.editor.selection = Selection::Room(id);
                }
            });
    }

    fn show_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Свойства");
                ui.separator();

                match self.editor.selection {
                    Selection::None => {
                        ui.label("Ничего не выбрано");
                    }
                    Selection::Wall(id) => {
                        // Set snapshot on first frame with this wall selected
                        if self.wall_edit_snapshot.is_none() {
                            if let Some(w) = self.project.walls.iter().find(|w| w.id == id) {
                                self.wall_edit_snapshot =
                                    Some((id, w.thickness, w.height_start, w.height_end));
                            }
                        }

                        if let Some(wall) = self.project.walls.iter_mut().find(|w| w.id == id) {
                            ui.label("Стена");
                            ui.add_space(8.0);

                            // Read-only: length
                            let length_mm = wall.length();
                            let length_label = if length_mm >= 1000.0 {
                                format!("{:.2} м ({:.0} мм)", length_mm / 1000.0, length_mm)
                            } else {
                                format!("{:.0} мм", length_mm)
                            };
                            ui.horizontal(|ui| {
                                ui.label("Длина:");
                                ui.label(length_label);
                            });

                            ui.add_space(4.0);

                            // Editable: thickness
                            ui.horizontal(|ui| {
                                ui.label("Толщина (мм):");
                                ui.add(
                                    egui::DragValue::new(&mut wall.thickness)
                                        .range(10.0..=1000.0)
                                        .speed(5.0),
                                );
                            });

                            // Editable: height_start
                            ui.horizontal(|ui| {
                                ui.label("Высота начала (мм):");
                                ui.add(
                                    egui::DragValue::new(&mut wall.height_start)
                                        .range(100.0..=10000.0)
                                        .speed(10.0),
                                );
                            });

                            // Editable: height_end
                            ui.horizontal(|ui| {
                                ui.label("Высота конца (мм):");
                                ui.add(
                                    egui::DragValue::new(&mut wall.height_end)
                                        .range(100.0..=10000.0)
                                        .speed(10.0),
                                );
                            });

                            ui.add_space(8.0);

                            // Read-only: gross area
                            let area_m2 = wall.gross_area() / 1_000_000.0;
                            ui.horizontal(|ui| {
                                ui.label("Площадь:");
                                ui.label(format!("{:.2} м²", area_m2));
                            });
                        } else {
                            ui.label("Стена не найдена");
                            self.editor.selection = Selection::None;
                        }
                    }
                    Selection::Opening(id) => {
                        // Set snapshot on first frame with this opening selected
                        if self.opening_edit_snapshot.is_none() {
                            if let Some(o) = self.project.openings.iter().find(|o| o.id == id) {
                                self.opening_edit_snapshot = Some((id, o.kind.clone()));
                            }
                        }

                        // Compute validation errors before mutable borrow
                        let errors: Vec<&str> = self
                            .project
                            .openings
                            .iter()
                            .find(|o| o.id == id)
                            .map(|o| self.opening_errors(o))
                            .unwrap_or_default();

                        // Get wall thickness for door depth display
                        let wall_thickness: Option<f64> = self
                            .project
                            .openings
                            .iter()
                            .find(|o| o.id == id)
                            .and_then(|o| o.wall_id)
                            .and_then(|wid| {
                                self.project.walls.iter().find(|w| w.id == wid)
                            })
                            .map(|w| w.thickness);

                        if let Some(opening) =
                            self.project.openings.iter_mut().find(|o| o.id == id)
                        {
                            let label = match &opening.kind {
                                OpeningKind::Door { .. } => "Дверь",
                                OpeningKind::Window { .. } => "Окно",
                            };
                            ui.label(label);
                            ui.add_space(8.0);

                            // Validation errors
                            if !errors.is_empty() {
                                for err in &errors {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(220, 60, 60),
                                        format!("⚠ {err}"),
                                    );
                                }
                                ui.add_space(4.0);
                            }

                            match &mut opening.kind {
                                OpeningKind::Door { height, width } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Высота (мм):");
                                        ui.add(
                                            egui::DragValue::new(height)
                                                .range(500.0..=3500.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Ширина (мм):");
                                        ui.add(
                                            egui::DragValue::new(width)
                                                .range(300.0..=3000.0)
                                                .speed(10.0),
                                        );
                                    });
                                    if let Some(thick) = wall_thickness {
                                        ui.horizontal(|ui| {
                                            ui.label("Глубина (мм):");
                                            ui.label(format!("{:.0}", thick));
                                        });
                                    }
                                }
                                OpeningKind::Window {
                                    height,
                                    width,
                                    sill_height,
                                    reveal_width,
                                } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Высота (мм):");
                                        ui.add(
                                            egui::DragValue::new(height)
                                                .range(200.0..=3000.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Ширина (мм):");
                                        ui.add(
                                            egui::DragValue::new(width)
                                                .range(200.0..=5000.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Подоконник (мм):");
                                        ui.add(
                                            egui::DragValue::new(sill_height)
                                                .range(0.0..=2500.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Откос (мм):");
                                        ui.add(
                                            egui::DragValue::new(reveal_width)
                                                .range(0.0..=500.0)
                                                .speed(5.0),
                                        );
                                    });
                                }
                            }
                        } else {
                            ui.label("Проём не найден");
                            self.editor.selection = Selection::None;
                        }
                    }
                    Selection::Room(id) => {
                        // Compute metrics before mutable borrow
                        let metrics = self
                            .project
                            .rooms
                            .iter()
                            .find(|r| r.id == id)
                            .and_then(|r| compute_room_metrics(r, &self.project.walls));

                        if let Some(room) =
                            self.project.rooms.iter_mut().find(|r| r.id == id)
                        {
                            ui.label("Комната");
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Название:");
                                if ui.text_edit_singleline(&mut room.name).changed() {
                                    self.dirty = true;
                                }
                            });

                            ui.add_space(4.0);

                            if let Some(m) = &metrics {
                                ui.horizontal(|ui| {
                                    ui.label("Площадь:");
                                    ui.label(format!("{:.2} м²", m.area / 1_000_000.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Периметр:");
                                    ui.label(format!("{:.2} м", m.perimeter / 1000.0));
                                });
                            }

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Стен в контуре:");
                                ui.label(format!("{}", room.wall_ids.len()));
                            });
                        } else {
                            ui.label("Комната не найдена");
                            self.editor.selection = Selection::None;
                        }
                    }
                }
            });
    }

    fn show_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .default_height(180.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Tab bar
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.bottom_tab,
                        BottomTab::PriceList,
                        "Прайс-лист",
                    );
                    ui.selectable_value(
                        &mut self.bottom_tab,
                        BottomTab::AssignedServices,
                        "Назначенные услуги",
                    );
                });
                ui.separator();

                match self.bottom_tab {
                    BottomTab::PriceList => self.show_price_list_tab(ui),
                    BottomTab::AssignedServices => self.show_assigned_services_tab(ui),
                }
            });
    }

    fn show_price_list_tab(&mut self, ui: &mut egui::Ui) {
        // Toolbar: Add / Delete / Import / Export
        ui.horizontal(|ui| {
            if ui.button("Добавить услугу").clicked() {
                self.price_list.services.push(ServiceTemplate::new(
                    "Новая услуга".to_string(),
                    UnitType::SquareMeter,
                    0.0,
                    TargetObjectType::Wall,
                ));
                self.selected_service_idx = Some(self.price_list.services.len() - 1);
            }

            let can_delete = self
                .selected_service_idx
                .map_or(false, |i| i < self.price_list.services.len());
            if ui
                .add_enabled(can_delete, egui::Button::new("Удалить"))
                .clicked()
            {
                if let Some(idx) = self.selected_service_idx {
                    self.price_list.services.remove(idx);
                    if self.price_list.services.is_empty() {
                        self.selected_service_idx = None;
                    } else {
                        self.selected_service_idx =
                            Some(idx.min(self.price_list.services.len() - 1));
                    }
                }
            }

            ui.separator();

            if ui.button("Импорт").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_title("Импорт прайс-листа")
                    .pick_file()
                {
                    match load_price_list(&path) {
                        Ok(pl) => {
                            self.price_list = pl;
                            self.selected_service_idx = None;
                            self.status_message = Some("Прайс-лист загружен".to_string());
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Ошибка импорта: {e}"));
                        }
                    }
                }
            }

            if ui.button("Экспорт").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_title("Экспорт прайс-листа")
                    .set_file_name(&format!("{}.json", self.price_list.name))
                    .save_file()
                {
                    match save_price_list_to(&self.price_list, &path) {
                        Ok(()) => {
                            self.status_message = Some("Прайс-лист сохранён".to_string());
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Ошибка экспорта: {e}"));
                        }
                    }
                }
            }

            // Show status message inline
            if let Some(msg) = &self.status_message {
                ui.separator();
                ui.label(msg);
            }
        });

        ui.add_space(4.0);

        if self.price_list.services.is_empty() {
            ui.label("Нет услуг. Нажмите «Добавить услугу».");
            return;
        }

        // Table header + rows
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("price_list_grid")
                .num_columns(5)
                .striped(true)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    // Header
                    ui.strong("");
                    ui.strong("Название");
                    ui.strong("Объект");
                    ui.strong("Ед. изм.");
                    ui.strong("Цена за ед.");
                    ui.end_row();

                    let mut new_sel = self.selected_service_idx;

                    for (i, svc) in self.price_list.services.iter_mut().enumerate() {
                        let is_selected = self.selected_service_idx == Some(i);

                        // Row selector
                        if ui
                            .selectable_label(is_selected, format!("{}", i + 1))
                            .clicked()
                        {
                            new_sel = Some(i);
                        }

                        // Name (editable)
                        ui.add(
                            egui::TextEdit::singleline(&mut svc.name).desired_width(180.0),
                        );

                        // Target object type (combo box)
                        egui::ComboBox::from_id_salt(format!("target_{i}"))
                            .selected_text(svc.target_type.label())
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for tt in TargetObjectType::ALL {
                                    ui.selectable_value(
                                        &mut svc.target_type,
                                        tt,
                                        tt.label(),
                                    );
                                }
                            });

                        // Unit type (combo box)
                        egui::ComboBox::from_id_salt(format!("unit_{i}"))
                            .selected_text(svc.unit_type.label())
                            .width(60.0)
                            .show_ui(ui, |ui| {
                                for ut in UnitType::ALL {
                                    ui.selectable_value(
                                        &mut svc.unit_type,
                                        ut,
                                        ut.label(),
                                    );
                                }
                            });

                        // Price per unit (editable)
                        ui.add(
                            egui::DragValue::new(&mut svc.price_per_unit)
                                .range(0.0..=f64::MAX)
                                .speed(10.0)
                                .suffix(" ₽"),
                        );

                        ui.end_row();
                    }

                    self.selected_service_idx = new_sel;
                });
        });
    }

    /// Determine the TargetObjectType for the current selection.
    fn selection_target_type(&self) -> Option<TargetObjectType> {
        match self.editor.selection {
            Selection::Wall(_) => Some(TargetObjectType::Wall),
            Selection::Opening(id) => {
                self.project.openings.iter().find(|o| o.id == id).map(|o| match &o.kind {
                    OpeningKind::Door { .. } => TargetObjectType::Door,
                    OpeningKind::Window { .. } => TargetObjectType::Window,
                })
            }
            Selection::Room(_) => Some(TargetObjectType::Room),
            Selection::None => None,
        }
    }

    /// Get the services map for the selected object, returning the object UUID and a mutable
    /// reference to its assigned services list.
    fn selected_object_services_mut(
        &mut self,
    ) -> Option<(uuid::Uuid, &mut Vec<AssignedService>)> {
        match self.editor.selection {
            Selection::Wall(id) => {
                let svcs = self.project.wall_services.entry(id).or_default();
                Some((id, svcs))
            }
            Selection::Opening(id) => {
                let svcs = self.project.opening_services.entry(id).or_default();
                Some((id, svcs))
            }
            Selection::Room(id) => {
                let svcs = self.project.room_services.entry(id).or_default();
                Some((id, svcs))
            }
            Selection::None => None,
        }
    }

    /// Label for the currently selected object.
    fn selection_label(&self) -> &'static str {
        match self.editor.selection {
            Selection::None => "—",
            Selection::Wall(_) => "Стена",
            Selection::Opening(id) => {
                match self.project.openings.iter().find(|o| o.id == id) {
                    Some(o) => match &o.kind {
                        OpeningKind::Door { .. } => "Дверь",
                        OpeningKind::Window { .. } => "Окно",
                    },
                    None => "Проём",
                }
            }
            Selection::Room(_) => "Комната",
        }
    }

    fn show_assigned_services_tab(&mut self, ui: &mut egui::Ui) {
        if self.editor.selection == Selection::None {
            ui.label("Выберите объект для назначения услуг.");
            return;
        }

        let target_type = self.selection_target_type();
        let label = self.selection_label();
        ui.label(format!("Объект: {label}"));
        ui.add_space(4.0);

        // Toolbar: Add Service / Remove
        let mut add_service_id: Option<uuid::Uuid> = None;
        let mut remove_idx: Option<usize> = None;

        ui.horizontal(|ui| {
            // "Add Service" dropdown filtered by target_type
            let available: Vec<&ServiceTemplate> = match target_type {
                Some(tt) => self
                    .price_list
                    .services
                    .iter()
                    .filter(|s| s.target_type == tt)
                    .collect(),
                None => Vec::new(),
            };

            let combo_label = if available.is_empty() {
                "Нет подходящих услуг".to_string()
            } else {
                "Добавить услугу...".to_string()
            };

            egui::ComboBox::from_id_salt("add_assigned_svc")
                .selected_text(combo_label)
                .show_ui(ui, |ui| {
                    for svc in &available {
                        if ui
                            .selectable_label(false, format!("{} ({})", svc.name, svc.unit_type.label()))
                            .clicked()
                        {
                            add_service_id = Some(svc.id);
                        }
                    }
                });
        });

        ui.add_space(4.0);

        // Get assigned services for the selected object and build display rows.
        // Collect read-only row data first, then render with editable prices.
        let rows: Vec<AssignedServiceRow> = match self.editor.selection {
            Selection::Wall(id) => {
                self.build_assigned_rows(self.project.wall_services.get(&id), id)
            }
            Selection::Opening(id) => {
                self.build_assigned_rows(self.project.opening_services.get(&id), id)
            }
            Selection::Room(id) => {
                self.build_assigned_rows(self.project.room_services.get(&id), id)
            }
            Selection::None => Vec::new(),
        };

        // Mutable copy of effective prices for DragValue editing.
        let mut prices: Vec<f64> = rows.iter().map(|r| r.effective_price).collect();
        // Track which row's "reset" button was clicked.
        let mut reset_idx: Option<usize> = None;

        if rows.is_empty() && add_service_id.is_none() {
            ui.label("Нет назначенных услуг.");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("assigned_services_grid")
                    .num_columns(7)
                    .striped(true)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.strong("Название");
                        ui.strong("Ед. изм.");
                        ui.strong("Цена за ед.");
                        ui.strong("Кол-во");
                        ui.strong("Стоимость");
                        ui.strong(""); // reset
                        ui.strong(""); // remove
                        ui.end_row();

                        for (i, row) in rows.iter().enumerate() {
                            ui.label(&row.name);
                            ui.label(&row.unit_label);

                            if row.valid {
                                // Editable price with DragValue
                                let dv = ui.add(
                                    egui::DragValue::new(&mut prices[i])
                                        .range(0.0..=f64::MAX)
                                        .speed(10.0)
                                        .suffix(" ₽"),
                                );
                                // Visual indicator: highlight when custom price is active
                                if row.has_custom {
                                    dv.highlight();
                                }

                                let cost = row.qty * prices[i];
                                ui.label(format!("{:.2}", row.qty));
                                ui.label(format!("{:.2} ₽", cost));

                                // Reset button (only shown when custom price)
                                if row.has_custom {
                                    if ui
                                        .small_button("↺")
                                        .on_hover_text("Сбросить к цене из прайс-листа")
                                        .clicked()
                                    {
                                        reset_idx = Some(i);
                                    }
                                } else {
                                    ui.label("");
                                }
                            } else {
                                ui.label("—");
                                ui.label("—");
                                ui.label("—");
                                ui.label("");
                            }

                            if ui.small_button("✕").clicked() {
                                remove_idx = Some(i);
                            }
                            ui.end_row();
                        }
                    });
            });
        }

        // Write back price changes to the assigned services.
        {
            let svcs = match self.editor.selection {
                Selection::Wall(id) => self.project.wall_services.get_mut(&id),
                Selection::Opening(id) => self.project.opening_services.get_mut(&id),
                Selection::Room(id) => self.project.room_services.get_mut(&id),
                Selection::None => None,
            };
            if let Some(svcs) = svcs {
                for (i, row) in rows.iter().enumerate() {
                    if !row.valid || i >= svcs.len() {
                        continue;
                    }
                    if let Some(idx) = reset_idx {
                        if idx == i {
                            svcs[i].custom_price = None;
                            self.dirty = true;
                            continue;
                        }
                    }
                    let new_price = prices[i];
                    if (new_price - row.template_price).abs() < 0.01 {
                        // Matches template — clear custom
                        if svcs[i].custom_price.is_some() {
                            svcs[i].custom_price = None;
                            self.dirty = true;
                        }
                    } else if (new_price - row.effective_price).abs() > 0.001 {
                        // User changed the price
                        svcs[i].custom_price = Some(new_price);
                        self.dirty = true;
                    }
                }
            }
        }

        // Apply add/remove mutations
        if let Some(tmpl_id) = add_service_id {
            if let Some((_obj_id, svcs)) = self.selected_object_services_mut() {
                svcs.push(AssignedService {
                    service_template_id: tmpl_id,
                    custom_price: None,
                });
                self.dirty = true;
            }
        }

        if let Some(idx) = remove_idx {
            if let Some((_obj_id, svcs)) = self.selected_object_services_mut() {
                if idx < svcs.len() {
                    svcs.remove(idx);
                    self.dirty = true;
                }
            }
        }
    }

    /// Compute quantity for a service assigned to the currently selected object.
    ///
    /// Returns quantity in natural units (pieces, m², or linear meters).
    fn compute_quantity(&self, unit_type: UnitType, obj_id: uuid::Uuid) -> f64 {
        match unit_type {
            UnitType::Piece => 1.0,
            UnitType::SquareMeter => {
                match self.editor.selection {
                    Selection::Wall(id) if id == obj_id => {
                        if let Some(wall) = self.project.walls.iter().find(|w| w.id == id) {
                            // Net area = gross area minus opening areas
                            let gross = wall.gross_area();
                            let openings_area: f64 = wall
                                .openings
                                .iter()
                                .filter_map(|oid| {
                                    self.project.openings.iter().find(|o| o.id == *oid)
                                })
                                .map(|o| o.kind.height() * o.kind.width())
                                .sum();
                            (gross - openings_area) / 1_000_000.0
                        } else {
                            0.0
                        }
                    }
                    Selection::Opening(id) if id == obj_id => {
                        if let Some(opening) =
                            self.project.openings.iter().find(|o| o.id == id)
                        {
                            match &opening.kind {
                                // Door: opening area
                                OpeningKind::Door { height, width } => {
                                    height * width / 1_000_000.0
                                }
                                // Window: reveal area = reveal_perimeter × reveal_width
                                OpeningKind::Window {
                                    height,
                                    width,
                                    reveal_width,
                                    ..
                                } => {
                                    let reveal_perimeter = 2.0 * height + 2.0 * width;
                                    reveal_perimeter * reveal_width / 1_000_000.0
                                }
                            }
                        } else {
                            0.0
                        }
                    }
                    Selection::Room(id) if id == obj_id => {
                        if let Some(room) = self.project.rooms.iter().find(|r| r.id == id) {
                            compute_room_metrics(room, &self.project.walls)
                                .map_or(0.0, |m| m.area / 1_000_000.0)
                        } else {
                            0.0
                        }
                    }
                    _ => 0.0,
                }
            }
            UnitType::LinearMeter => {
                match self.editor.selection {
                    Selection::Wall(id) if id == obj_id => {
                        self.project
                            .walls
                            .iter()
                            .find(|w| w.id == id)
                            .map_or(0.0, |w| w.length() / 1000.0)
                    }
                    Selection::Opening(id) if id == obj_id => {
                        if let Some(opening) =
                            self.project.openings.iter().find(|o| o.id == id)
                        {
                            match &opening.kind {
                                // Door: 2 × height + width (no threshold)
                                OpeningKind::Door { height, width } => {
                                    (2.0 * height + width) / 1000.0
                                }
                                // Window: reveal perimeter = 2 × height + 2 × width
                                OpeningKind::Window { height, width, .. } => {
                                    (2.0 * height + 2.0 * width) / 1000.0
                                }
                            }
                        } else {
                            0.0
                        }
                    }
                    Selection::Room(id) if id == obj_id => {
                        if let Some(room) = self.project.rooms.iter().find(|r| r.id == id) {
                            compute_room_metrics(room, &self.project.walls)
                                .map_or(0.0, |m| m.perimeter / 1000.0)
                        } else {
                            0.0
                        }
                    }
                    _ => 0.0,
                }
            }
        }
    }

    /// Build display rows for assigned services on the object with the given id.
    fn build_assigned_rows(
        &self,
        assigned: Option<&Vec<AssignedService>>,
        obj_id: uuid::Uuid,
    ) -> Vec<AssignedServiceRow> {
        let Some(assigned) = assigned else {
            return Vec::new();
        };
        assigned
            .iter()
            .map(|a| {
                let tmpl = self
                    .price_list
                    .services
                    .iter()
                    .find(|s| s.id == a.service_template_id);
                match tmpl {
                    Some(t) => {
                        let effective = a.custom_price.unwrap_or(t.price_per_unit);
                        let qty = self.compute_quantity(t.unit_type, obj_id);
                        AssignedServiceRow {
                            name: t.name.clone(),
                            unit_label: t.unit_type.label().to_string(),
                            template_price: t.price_per_unit,
                            effective_price: effective,
                            has_custom: a.custom_price.is_some(),
                            qty,
                            valid: true,
                        }
                    }
                    None => AssignedServiceRow {
                        name: "⚠ Услуга удалена".to_string(),
                        unit_label: "—".to_string(),
                        template_price: 0.0,
                        effective_price: 0.0,
                        has_custom: false,
                        qty: 0.0,
                        valid: false,
                    },
                }
            })
            .collect()
    }

    fn show_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

            let rect = response.rect;

            // --- Pan: middle mouse drag ---
            if response.dragged_by(egui::PointerButton::Middle) {
                self.editor.canvas.pan(response.drag_delta());
            }

            // --- Pan: Space + LMB drag ---
            let space_held = ui.input(|i| i.key_down(egui::Key::Space));
            if space_held && response.dragged_by(egui::PointerButton::Primary) {
                self.editor.canvas.pan(response.drag_delta());
            }

            // --- Zoom: mouse wheel toward cursor ---
            if response.hovered() {
                let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_y != 0.0 {
                    let factor = 1.1_f32.powf(scroll_y / 24.0);
                    let cursor = response.hover_pos().unwrap_or(rect.center());
                    self.editor.canvas.zoom_toward(cursor, rect.center(), factor);
                }
            }

            // Background
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(45, 45, 48));

            // Grid
            self.editor.canvas.draw_grid(&painter, rect);

            // Track cursor position in world coordinates
            if let Some(pos) = response.hover_pos() {
                let world = self.editor.canvas.screen_to_world(pos, rect.center());
                self.editor.canvas.cursor_world_pos = Some(world);
            } else {
                self.editor.canvas.cursor_world_pos = None;
            }

            // --- Wall tool input handling ---
            let shift_held = ui.input(|i| i.modifiers.shift);
            if self.editor.active_tool == EditorTool::Wall {
                // Update preview position (snapped) every frame
                if let Some(hover) = response.hover_pos() {
                    let world = self.editor.canvas.screen_to_world(hover, rect.center());
                    let world_pt = Point2D::new(world.x as f64, world.y as f64);
                    let snap_result = snap(
                        world_pt,
                        self.editor.canvas.grid_step,
                        self.editor.canvas.zoom,
                        &self.project.walls,
                        shift_held,
                    );
                    self.editor.wall_tool.preview_end = Some(snap_result.position);
                } else {
                    self.editor.wall_tool.preview_end = None;
                }

                // Double-click finishes the chain
                if response.double_clicked() && !space_held {
                    self.editor.wall_tool.reset();
                }
                // Single click: place point / create wall
                else if response.clicked() && !space_held {
                    if let Some(snapped) = self.editor.wall_tool.preview_end {
                        match self.editor.wall_tool.state.clone() {
                            WallToolState::Idle => {
                                self.editor.wall_tool.chain_start = Some(snapped);
                                self.editor.wall_tool.state =
                                    WallToolState::Drawing { start: snapped };
                            }
                            WallToolState::Drawing { start } => {
                                // Check if clicking on chain start to close contour
                                let closing = if let Some(chain_start) =
                                    self.editor.wall_tool.chain_start
                                {
                                    let snap_radius =
                                        15.0_f64 / self.editor.canvas.zoom as f64;
                                    snapped.distance_to(chain_start) < snap_radius
                                        && start.distance_to(chain_start) > 1.0
                                } else {
                                    false
                                };

                                if closing {
                                    // Close contour: create wall back to chain start
                                    let chain_start =
                                        self.editor.wall_tool.chain_start.unwrap();
                                    let wall = Wall::new(start, chain_start);
                                    self.flush_property_edits();
                                    self.history.push(
                                        Box::new(AddWallCommand { wall }),
                                        &mut self.project,
                                    );
                                    self.editor.wall_tool.reset();
                                } else if start.distance_to(snapped) > 1.0 {
                                    // Create wall and chain from its endpoint
                                    let wall = Wall::new(start, snapped);
                                    self.flush_property_edits();
                                    self.history.push(
                                        Box::new(AddWallCommand { wall }),
                                        &mut self.project,
                                    );
                                    self.editor.wall_tool.chain_from(snapped);
                                }
                            }
                        }
                    }
                }

                // Escape finishes chain / cancels drawing
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.editor.wall_tool.reset();
                }
            }

            // --- Select tool input handling ---
            if self.editor.active_tool == EditorTool::Select {
                // Click to select (openings have priority over walls)
                if response.clicked() && !space_held {
                    if let Some(hover) = response.hover_pos() {
                        let world = self.editor.canvas.screen_to_world(hover, rect.center());
                        let click_pt = Point2D::new(world.x as f64, world.y as f64);
                        let hit_tolerance = 10.0_f64 / self.editor.canvas.zoom as f64;

                        // Check openings first (they sit on top of walls)
                        let mut best_opening: Option<(uuid::Uuid, f64)> = None;
                        for opening in &self.project.openings {
                            if let Some(wid) = opening.wall_id {
                                if let Some(wall) =
                                    self.project.walls.iter().find(|w| w.id == wid)
                                {
                                    let wall_len = wall.length();
                                    if wall_len < 1.0 {
                                        continue;
                                    }
                                    let t = (opening.offset_along_wall / wall_len)
                                        .clamp(0.0, 1.0);
                                    let cx =
                                        wall.start.x + (wall.end.x - wall.start.x) * t;
                                    let cy =
                                        wall.start.y + (wall.end.y - wall.start.y) * t;
                                    let dist =
                                        click_pt.distance_to(Point2D::new(cx, cy));
                                    let threshold =
                                        opening.kind.width() / 2.0 + hit_tolerance;
                                    if dist < threshold {
                                        if best_opening.is_none()
                                            || dist < best_opening.unwrap().1
                                        {
                                            best_opening = Some((opening.id, dist));
                                        }
                                    }
                                }
                            }
                        }

                        if let Some((id, _)) = best_opening {
                            self.editor.selection = Selection::Opening(id);
                        } else {
                            // Check walls
                            let mut best_wall: Option<(uuid::Uuid, f64)> = None;
                            for wall in &self.project.walls {
                                let dist =
                                    click_pt.distance_to_segment(wall.start, wall.end);
                                let threshold = wall.thickness / 2.0 + hit_tolerance;
                                if dist < threshold {
                                    if best_wall.is_none()
                                        || dist < best_wall.unwrap().1
                                    {
                                        best_wall = Some((wall.id, dist));
                                    }
                                }
                            }
                            self.editor.selection = match best_wall {
                                Some((id, _)) => Selection::Wall(id),
                                None => Selection::None,
                            };
                        }
                    }
                }

                // Drag selected opening along/between walls
                if response.dragged_by(egui::PointerButton::Primary) && !space_held {
                    if let Selection::Opening(oid) = self.editor.selection {
                        if let Some(hover) = response.hover_pos() {
                            let world =
                                self.editor.canvas.screen_to_world(hover, rect.center());
                            let cursor_pt =
                                Point2D::new(world.x as f64, world.y as f64);
                            let hit_tolerance =
                                10.0_f64 / self.editor.canvas.zoom as f64;

                            // Find nearest wall under cursor
                            let mut best: Option<(uuid::Uuid, f64, f64)> = None;
                            for wall in &self.project.walls {
                                let dist = cursor_pt
                                    .distance_to_segment(wall.start, wall.end);
                                let threshold = wall.thickness / 2.0 + hit_tolerance;
                                if dist < threshold {
                                    if best.is_none() || dist < best.unwrap().1 {
                                        let (t, _) = cursor_pt
                                            .project_onto_segment(wall.start, wall.end);
                                        let offset = t * wall.length();
                                        best = Some((wall.id, dist, offset));
                                    }
                                }
                            }

                            // Get old wall_id before mutation
                            let old_wall_id = self
                                .project
                                .openings
                                .iter()
                                .find(|o| o.id == oid)
                                .and_then(|o| o.wall_id);

                            if let Some((new_wall_id, _, new_offset)) = best {
                                // Snap to wall
                                if let Some(opening) = self
                                    .project
                                    .openings
                                    .iter_mut()
                                    .find(|o| o.id == oid)
                                {
                                    opening.wall_id = Some(new_wall_id);
                                    opening.offset_along_wall = new_offset;
                                }
                                // Update wall opening lists if wall changed
                                if old_wall_id != Some(new_wall_id) {
                                    if let Some(prev_wid) = old_wall_id {
                                        if let Some(w) = self
                                            .project
                                            .walls
                                            .iter_mut()
                                            .find(|w| w.id == prev_wid)
                                        {
                                            w.openings.retain(|id| *id != oid);
                                        }
                                    }
                                    if let Some(w) = self
                                        .project
                                        .walls
                                        .iter_mut()
                                        .find(|w| w.id == new_wall_id)
                                    {
                                        if !w.openings.contains(&oid) {
                                            w.openings.push(oid);
                                        }
                                    }
                                }
                            } else {
                                // Not over any wall — detach opening
                                if let Some(opening) = self
                                    .project
                                    .openings
                                    .iter_mut()
                                    .find(|o| o.id == oid)
                                {
                                    opening.wall_id = None;
                                }
                                if let Some(prev_wid) = old_wall_id {
                                    if let Some(w) = self
                                        .project
                                        .walls
                                        .iter_mut()
                                        .find(|w| w.id == prev_wid)
                                    {
                                        w.openings.retain(|id| *id != oid);
                                    }
                                }
                            }
                        }
                    }
                }

                // Escape to deselect
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.editor.selection = Selection::None;
                }

                // Delete key to remove selected wall or opening
                if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                    match self.editor.selection {
                        Selection::Wall(id) => {
                            self.flush_property_edits();
                            if let Some(cmd) = RemoveWallCommand::new(id, &self.project) {
                                self.history
                                    .push(Box::new(cmd), &mut self.project);
                            }
                            self.editor.selection = Selection::None;
                        }
                        Selection::Opening(id) => {
                            self.flush_property_edits();
                            if let Some(cmd) = RemoveOpeningCommand::new(id, &self.project) {
                                self.history
                                    .push(Box::new(cmd), &mut self.project);
                            }
                            self.editor.selection = Selection::None;
                        }
                        _ => {}
                    }
                }
            }

            // --- Opening tool (Door / Window) input handling ---
            if self.editor.active_tool == EditorTool::Door
                || self.editor.active_tool == EditorTool::Window
            {
                // Update hover preview: find wall under cursor
                self.editor.opening_tool.hover_wall_id = None;
                if let Some(hover) = response.hover_pos() {
                    let world = self.editor.canvas.screen_to_world(hover, rect.center());
                    let cursor_pt = Point2D::new(world.x as f64, world.y as f64);
                    let hit_tolerance = 10.0_f64 / self.editor.canvas.zoom as f64;

                    let mut best: Option<(uuid::Uuid, f64, f64)> = None; // (wall_id, dist, offset)
                    for wall in &self.project.walls {
                        let dist = cursor_pt.distance_to_segment(wall.start, wall.end);
                        let threshold = wall.thickness / 2.0 + hit_tolerance;
                        if dist < threshold {
                            if best.is_none() || dist < best.unwrap().1 {
                                let (t, _proj) =
                                    cursor_pt.project_onto_segment(wall.start, wall.end);
                                let offset = t * wall.length();
                                best = Some((wall.id, dist, offset));
                            }
                        }
                    }

                    if let Some((wall_id, _dist, offset)) = best {
                        self.editor.opening_tool.hover_wall_id = Some(wall_id);
                        self.editor.opening_tool.hover_offset = offset;
                    }
                }

                // Click to place opening on the hovered wall
                if response.clicked() && !space_held {
                    if let Some(wall_id) = self.editor.opening_tool.hover_wall_id {
                        let offset = self.editor.opening_tool.hover_offset;
                        let kind = if self.editor.active_tool == EditorTool::Door {
                            OpeningKind::default_door()
                        } else {
                            OpeningKind::default_window()
                        };
                        let opening = Opening::new(kind, Some(wall_id), offset);
                        let opening_id = opening.id;
                        self.flush_property_edits();
                        self.history.push(
                            Box::new(AddOpeningCommand { opening }),
                            &mut self.project,
                        );
                        self.editor.selection = Selection::Opening(opening_id);
                    }
                }
            }

            // --- Detect rooms from wall graph ---
            let graph = WallGraph::build(&self.project.walls);
            let new_rooms = graph.detect_rooms(&self.project.walls);
            self.merge_rooms(new_rooms);

            // --- Render detected rooms (before walls, so they appear underneath) ---
            self.draw_rooms(&painter, rect);

            // --- Render existing walls ---
            self.draw_walls(&painter, rect);

            // --- Render existing openings ---
            self.draw_openings(&painter, rect);

            // --- Render wall tool preview ---
            if self.editor.active_tool == EditorTool::Wall {
                self.draw_wall_preview(&painter, rect);
            }

            // --- Render opening tool preview ---
            if (self.editor.active_tool == EditorTool::Door
                || self.editor.active_tool == EditorTool::Window)
                && self.editor.opening_tool.hover_wall_id.is_some()
            {
                self.draw_opening_preview(&painter, rect);
            }

            // Tool hint (only when idle and no walls exist)
            if self.project.walls.is_empty() {
                let tool_hint = match self.editor.active_tool {
                    EditorTool::Select => "Режим выбора — кликните на объект",
                    EditorTool::Wall => match self.editor.wall_tool.state {
                        WallToolState::Idle => "Кликните для начальной точки стены",
                        WallToolState::Drawing { .. } => "Кликните для конечной точки стены",
                    },
                    EditorTool::Door => "Режим двери — перетащите на стену",
                    EditorTool::Window => "Режим окна — перетащите на стену",
                };
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    tool_hint,
                    egui::FontId::proportional(16.0),
                    egui::Color32::from_rgb(120, 120, 120),
                );
            }

            // Status bar with coordinates and zoom level
            if let Some(pos) = self.editor.canvas.cursor_world_pos {
                let zoom_pct = self.editor.canvas.zoom * 200.0; // 0.5 default = 100%
                let status = format!(
                    "X: {:.0} мм  Y: {:.0} мм  |  Масштаб: {:.0}%",
                    pos.x, pos.y, zoom_pct
                );
                painter.text(
                    egui::pos2(rect.left() + 8.0, rect.bottom() - 20.0),
                    egui::Align2::LEFT_CENTER,
                    status,
                    egui::FontId::monospace(12.0),
                    egui::Color32::from_rgb(180, 180, 180),
                );
            }
        });
    }

    /// Draw all existing walls on the canvas.
    fn draw_walls(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let wall_fill = egui::Color32::from_rgb(140, 140, 145);
        let wall_outline = egui::Color32::from_rgb(40, 40, 42);
        let endpoint_color = egui::Color32::from_rgb(200, 200, 220);
        let dim_color = egui::Color32::from_rgb(220, 220, 230);

        let selected_id = match self.editor.selection {
            Selection::Wall(id) => Some(id),
            _ => None,
        };

        for wall in &self.project.walls {
            let is_selected = selected_id == Some(wall.id);
            let fill = if is_selected {
                egui::Color32::from_rgb(100, 160, 220)
            } else {
                wall_fill
            };
            let outline = if is_selected {
                egui::Color32::from_rgb(60, 120, 200)
            } else {
                wall_outline
            };

            let start_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.start.x as f32, wall.start.y as f32),
                center,
            );
            let end_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.end.x as f32, wall.end.y as f32),
                center,
            );

            // Compute perpendicular offset for wall thickness
            let dx = end_screen.x - start_screen.x;
            let dy = end_screen.y - start_screen.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.1 {
                continue;
            }

            let half_thick_screen = (wall.thickness as f32 * self.editor.canvas.zoom) / 2.0;
            let nx = -dy / len * half_thick_screen;
            let ny = dx / len * half_thick_screen;

            let corners = [
                egui::pos2(start_screen.x + nx, start_screen.y + ny),
                egui::pos2(end_screen.x + nx, end_screen.y + ny),
                egui::pos2(end_screen.x - nx, end_screen.y - ny),
                egui::pos2(start_screen.x - nx, start_screen.y - ny),
            ];

            // Filled rectangle
            let outline_width = if is_selected { 2.0 } else { 1.0 };
            painter.add(egui::Shape::convex_polygon(
                corners.to_vec(),
                fill,
                egui::Stroke::new(outline_width, outline),
            ));

            // Endpoints
            let ep_radius = if is_selected { 4.0 } else { 3.0 };
            painter.circle_filled(start_screen, ep_radius, endpoint_color);
            painter.circle_filled(end_screen, ep_radius, endpoint_color);

            // Dimension label (wall length) at midpoint, offset perpendicular to wall
            let length_mm = wall.length();
            if length_mm > 1.0 {
                let mid = egui::pos2(
                    (start_screen.x + end_screen.x) / 2.0 + nx * 0.6,
                    (start_screen.y + end_screen.y) / 2.0 + ny * 0.6,
                );
                let label = if length_mm >= 1000.0 {
                    format!("{:.2} м", length_mm / 1000.0)
                } else {
                    format!("{:.0} мм", length_mm)
                };
                painter.text(
                    mid,
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(11.0),
                    dim_color,
                );
            }
        }
    }

    /// Draw the wall tool preview (start point marker + preview line + chain start).
    fn draw_wall_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let preview_color = egui::Color32::from_rgba_premultiplied(100, 180, 255, 180);
        let start_marker_color = egui::Color32::from_rgb(100, 180, 255);
        let chain_start_color = egui::Color32::from_rgb(60, 220, 120);

        // Chain start marker (green, so user knows where to click to close contour)
        if let Some(chain_start) = self.editor.wall_tool.chain_start {
            if let WallToolState::Drawing { start } = &self.editor.wall_tool.state {
                // Only show if we've moved past the first segment
                if start.distance_to(chain_start) > 1.0 {
                    let cs_screen = self.editor.canvas.world_to_screen(
                        egui::pos2(chain_start.x as f32, chain_start.y as f32),
                        center,
                    );
                    painter.circle_filled(cs_screen, 6.0, chain_start_color);
                }
            }
        }

        if let WallToolState::Drawing { start } = &self.editor.wall_tool.state {
            let start_screen = self.editor.canvas.world_to_screen(
                egui::pos2(start.x as f32, start.y as f32),
                center,
            );

            // Current segment start point marker
            painter.circle_filled(start_screen, 5.0, start_marker_color);

            // Preview line to current cursor
            if let Some(end) = self.editor.wall_tool.preview_end {
                let end_screen = self.editor.canvas.world_to_screen(
                    egui::pos2(end.x as f32, end.y as f32),
                    center,
                );
                painter.line_segment(
                    [start_screen, end_screen],
                    egui::Stroke::new(2.0, preview_color),
                );

                // Preview length label
                let length_mm = start.distance_to(end);
                if length_mm > 1.0 {
                    let mid = egui::pos2(
                        (start_screen.x + end_screen.x) / 2.0,
                        (start_screen.y + end_screen.y) / 2.0 - 12.0,
                    );
                    let label = if length_mm >= 1000.0 {
                        format!("{:.2} м", length_mm / 1000.0)
                    } else {
                        format!("{:.0} мм", length_mm)
                    };
                    painter.text(
                        mid,
                        egui::Align2::CENTER_BOTTOM,
                        label,
                        egui::FontId::proportional(12.0),
                        preview_color,
                    );
                }
            }
        }

        // Show snap indicator at preview point
        if let Some(end) = self.editor.wall_tool.preview_end {
            let end_screen = self.editor.canvas.world_to_screen(
                egui::pos2(end.x as f32, end.y as f32),
                center,
            );
            painter.circle_stroke(
                end_screen,
                4.0,
                egui::Stroke::new(1.5, start_marker_color),
            );
        }
    }

    /// Draw all placed openings (doors and windows) on the canvas.
    fn draw_openings(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let canvas_bg = egui::Color32::from_rgb(45, 45, 48);

        let selected_opening_id = match self.editor.selection {
            Selection::Opening(id) => Some(id),
            _ => None,
        };

        for opening in &self.project.openings {
            let wall = match opening.wall_id {
                Some(wid) => match self.project.walls.iter().find(|w| w.id == wid) {
                    Some(w) => w,
                    None => continue,
                },
                None => {
                    // Unattached opening — render red marker at canvas origin as fallback
                    let origin = self.editor.canvas.world_to_screen(egui::pos2(0.0, 0.0), center);
                    painter.circle_filled(origin, 6.0, egui::Color32::from_rgb(220, 50, 50));
                    painter.text(
                        egui::pos2(origin.x, origin.y - 10.0),
                        egui::Align2::CENTER_BOTTOM,
                        "⚠",
                        egui::FontId::proportional(14.0),
                        egui::Color32::from_rgb(220, 50, 50),
                    );
                    continue;
                }
            };

            let wall_len = wall.length();
            if wall_len < 1.0 {
                continue;
            }

            let opening_width = opening.kind.width();
            let half_w = opening_width / 2.0;
            let t_left = ((opening.offset_along_wall - half_w) / wall_len).clamp(0.0, 1.0);
            let t_right = ((opening.offset_along_wall + half_w) / wall_len).clamp(0.0, 1.0);

            let lerp_wall = |t: f64| -> egui::Pos2 {
                let wx = wall.start.x + (wall.end.x - wall.start.x) * t;
                let wy = wall.start.y + (wall.end.y - wall.start.y) * t;
                self.editor
                    .canvas
                    .world_to_screen(egui::pos2(wx as f32, wy as f32), center)
            };

            let p_left = lerp_wall(t_left);
            let p_right = lerp_wall(t_right);

            // Wall direction in screen space for perpendicular computation
            let start_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.start.x as f32, wall.start.y as f32),
                center,
            );
            let end_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.end.x as f32, wall.end.y as f32),
                center,
            );
            let dx = end_screen.x - start_screen.x;
            let dy = end_screen.y - start_screen.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.1 {
                continue;
            }

            let half_thick = (wall.thickness as f32 * self.editor.canvas.zoom) / 2.0;
            let nx = -dy / len * half_thick;
            let ny = dx / len * half_thick;

            let is_selected = selected_opening_id == Some(opening.id);

            // Draw gap in wall (cover with background color, slightly oversized)
            let gap_corners = [
                egui::pos2(p_left.x + nx * 1.1, p_left.y + ny * 1.1),
                egui::pos2(p_right.x + nx * 1.1, p_right.y + ny * 1.1),
                egui::pos2(p_right.x - nx * 1.1, p_right.y - ny * 1.1),
                egui::pos2(p_left.x - nx * 1.1, p_left.y - ny * 1.1),
            ];
            painter.add(egui::Shape::convex_polygon(
                gap_corners.to_vec(),
                canvas_bg,
                egui::Stroke::NONE,
            ));

            match &opening.kind {
                OpeningKind::Door { .. } => {
                    let color = if is_selected {
                        egui::Color32::from_rgb(240, 180, 80)
                    } else {
                        egui::Color32::from_rgb(180, 120, 60)
                    };
                    let stroke_w = if is_selected { 2.0 } else { 1.5 };

                    // Door leaf line (from hinge to swing edge)
                    painter.line_segment(
                        [p_left, p_right],
                        egui::Stroke::new(stroke_w, color),
                    );

                    // Swing arc: quarter circle from hinge (p_left), radius = opening width
                    let arc_r = ((p_right.x - p_left.x).powi(2)
                        + (p_right.y - p_left.y).powi(2))
                    .sqrt();
                    if arc_r > 1.0 {
                        // Unit vector along wall and perpendicular
                        let ux = (p_right.x - p_left.x) / arc_r;
                        let uy = (p_right.y - p_left.y) / arc_r;
                        let px = -uy;
                        let py = ux;

                        let n_seg = 16;
                        let mut pts = Vec::with_capacity(n_seg + 1);
                        for i in 0..=n_seg {
                            let a = (i as f32 / n_seg as f32) * std::f32::consts::FRAC_PI_2;
                            let d_x = ux * a.cos() + px * a.sin();
                            let d_y = uy * a.cos() + py * a.sin();
                            pts.push(egui::pos2(
                                p_left.x + d_x * arc_r,
                                p_left.y + d_y * arc_r,
                            ));
                        }
                        for i in 0..n_seg {
                            painter.line_segment(
                                [pts[i], pts[i + 1]],
                                egui::Stroke::new(stroke_w, color),
                            );
                        }
                    }
                }
                OpeningKind::Window { .. } => {
                    let color = if is_selected {
                        egui::Color32::from_rgb(120, 210, 255)
                    } else {
                        egui::Color32::from_rgb(80, 160, 220)
                    };
                    let stroke_w = if is_selected { 2.0 } else { 1.5 };

                    // Two parallel lines representing glass panes
                    for sign in [-0.3_f32, 0.3_f32] {
                        let ox = nx * sign;
                        let oy = ny * sign;
                        painter.line_segment(
                            [
                                egui::pos2(p_left.x + ox, p_left.y + oy),
                                egui::pos2(p_right.x + ox, p_right.y + oy),
                            ],
                            egui::Stroke::new(stroke_w, color),
                        );
                    }

                    // Sill ticks at edges
                    for p in [p_left, p_right] {
                        painter.line_segment(
                            [
                                egui::pos2(p.x + nx * 0.3, p.y + ny * 0.3),
                                egui::pos2(p.x - nx * 0.3, p.y - ny * 0.3),
                            ],
                            egui::Stroke::new(stroke_w, color),
                        );
                    }
                }
            }
        }
    }

    /// Draw detected rooms as semi-transparent filled polygons with names at centroids.
    fn draw_rooms(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();

        // Distinct room colors (semi-transparent)
        const ROOM_COLORS: &[(u8, u8, u8)] = &[
            (70, 130, 180),  // steel blue
            (60, 179, 113),  // medium sea green
            (218, 165, 32),  // goldenrod
            (178, 102, 178), // orchid
            (205, 92, 92),   // indian red
            (72, 209, 204),  // medium turquoise
            (244, 164, 96),  // sandy brown
            (123, 104, 238), // medium slate blue
        ];

        for (i, room) in self.project.rooms.iter().enumerate() {
            let metrics = match compute_room_metrics(room, &self.project.walls) {
                Some(m) => m,
                None => continue,
            };

            if metrics.inner_polygon.len() < 3 {
                continue;
            }

            let (r, g, b) = ROOM_COLORS[i % ROOM_COLORS.len()];
            let fill = egui::Color32::from_rgba_unmultiplied(r, g, b, 40);

            // Convert inner polygon to screen coordinates
            let screen_pts: Vec<egui::Pos2> = metrics
                .inner_polygon
                .iter()
                .map(|p| {
                    self.editor
                        .canvas
                        .world_to_screen(egui::pos2(p.x as f32, p.y as f32), center)
                })
                .collect();

            painter.add(egui::Shape::convex_polygon(
                screen_pts.clone(),
                fill,
                egui::Stroke::NONE,
            ));

            // Centroid for the name label
            let cx: f32 = screen_pts.iter().map(|p| p.x).sum::<f32>() / screen_pts.len() as f32;
            let cy: f32 = screen_pts.iter().map(|p| p.y).sum::<f32>() / screen_pts.len() as f32;

            let label_color = egui::Color32::from_rgb(r, g, b);
            painter.text(
                egui::pos2(cx, cy),
                egui::Align2::CENTER_CENTER,
                &room.name,
                egui::FontId::proportional(13.0),
                label_color,
            );

            // Area label below the name
            let area_m2 = metrics.area / 1_000_000.0;
            painter.text(
                egui::pos2(cx, cy + 16.0),
                egui::Align2::CENTER_CENTER,
                format!("{:.1} м²", area_m2),
                egui::FontId::proportional(11.0),
                label_color,
            );
        }
    }

    /// Draw a preview of the opening being placed (door/window ghost on the hovered wall).
    fn draw_opening_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let wall_id = match self.editor.opening_tool.hover_wall_id {
            Some(id) => id,
            None => return,
        };
        let wall = match self.project.walls.iter().find(|w| w.id == wall_id) {
            Some(w) => w,
            None => return,
        };

        let opening_width = if self.editor.active_tool == EditorTool::Door {
            900.0 // default door width
        } else {
            1200.0 // default window width
        };

        let offset = self.editor.opening_tool.hover_offset;
        let wall_len = wall.length();
        if wall_len < 1.0 {
            return;
        }

        // Compute positions along the wall for the opening edges
        let half_w = opening_width / 2.0;
        let t_center = offset / wall_len;
        let t_left = ((offset - half_w) / wall_len).clamp(0.0, 1.0);
        let t_right = ((offset + half_w) / wall_len).clamp(0.0, 1.0);

        let lerp_wall = |t: f64| -> egui::Pos2 {
            let wx = wall.start.x + (wall.end.x - wall.start.x) * t;
            let wy = wall.start.y + (wall.end.y - wall.start.y) * t;
            self.editor.canvas.world_to_screen(egui::pos2(wx as f32, wy as f32), center)
        };

        let p_left = lerp_wall(t_left);
        let p_right = lerp_wall(t_right);
        let p_center = lerp_wall(t_center);

        // Perpendicular for thickness
        let dx = p_right.x - p_left.x;
        let dy = p_right.y - p_left.y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len < 0.5 {
            return;
        }

        let half_thick_screen = (wall.thickness as f32 * self.editor.canvas.zoom) / 2.0;
        let nx = -dy / seg_len * half_thick_screen;
        let ny = dx / seg_len * half_thick_screen;

        let preview_color = if self.editor.active_tool == EditorTool::Door {
            egui::Color32::from_rgba_premultiplied(180, 120, 60, 160)
        } else {
            egui::Color32::from_rgba_premultiplied(80, 160, 220, 160)
        };

        // Draw the opening as a rectangle on the wall
        let corners = [
            egui::pos2(p_left.x + nx, p_left.y + ny),
            egui::pos2(p_right.x + nx, p_right.y + ny),
            egui::pos2(p_right.x - nx, p_right.y - ny),
            egui::pos2(p_left.x - nx, p_left.y - ny),
        ];

        painter.add(egui::Shape::convex_polygon(
            corners.to_vec(),
            preview_color,
            egui::Stroke::new(2.0, preview_color),
        ));

        // Label at center
        let label = if self.editor.active_tool == EditorTool::Door {
            "Дверь"
        } else {
            "Окно"
        };
        painter.text(
            egui::pos2(p_center.x, p_center.y - half_thick_screen - 10.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            egui::FontId::proportional(11.0),
            preview_color,
        );
    }
}
