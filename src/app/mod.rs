use eframe::egui;

use crate::editor::EditorState;
use crate::editor::EditorTool;
use history::History;
use crate::model::{PriceList, Project, ProjectDefaults, Room, WallSide};
use crate::persistence::{list_project_entries, load_project, save_project, ProjectEntry};

mod history;
mod canvas;
mod canvas_draw;
mod price_list;
mod project_list;
mod properties_panel;
mod property_edits;
mod service_picker;
mod services_panel;
mod toolbar;

/// Section color palette shared across canvas rendering, property editors, and services panel.
const SECTION_COLORS: &[(u8, u8, u8)] = &[
    (100, 180, 240),
    (240, 160, 100),
    (100, 220, 140),
    (220, 120, 220),
    (240, 220, 100),
    (120, 220, 220),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppScreen {
    ProjectList,
    Editor,
}

#[derive(Debug, Clone)]
enum ServiceTarget {
    WallSide { wall_id: uuid::Uuid, side: WallSide, section_index: usize },
    Opening { opening_id: uuid::Uuid },
    Room { room_id: uuid::Uuid },
}

pub struct App {
    screen: AppScreen,
    project_entries: Vec<ProjectEntry>,
    project_list_selection: Option<usize>,
    new_project_name: String,
    confirm_delete: Option<usize>,
    show_new_project_dialog: bool,
    new_project_defaults: ProjectDefaults,
    show_project_settings: bool,

    pub project: Project,
    pub editor: EditorState,
    pub history: History,
    edit_snapshot_version: Option<u64>,
    pub price_list: PriceList,
    selected_service_idx: Option<usize>,
    status_message: Option<String>,
    show_price_list_window: bool,
    show_service_picker: bool,
    service_picker_filter: String,
    service_picker_target: Option<ServiceTarget>,
    price_list_filter: String,
    last_saved_version: u64,
    label_scale: f32,
    rooms_version: u64,
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
            new_project_defaults: ProjectDefaults::default(),
            show_project_settings: false,

            project: Project::new("Новый проект".to_string()),
            editor: EditorState::default(),
            history: History::new(),
            edit_snapshot_version: None,
            price_list: PriceList::new("Прайс-лист".to_string()),
            selected_service_idx: None,
            status_message: None,
            show_price_list_window: false,
            show_service_picker: false,
            service_picker_filter: String::new(),
            service_picker_target: None,
            price_list_filter: String::new(),
            last_saved_version: 0,
            label_scale: 1.0,
            rooms_version: 0,
        }
    }

    fn refresh_project_list(&mut self) {
        self.project_entries = list_project_entries().unwrap_or_default();
        self.project_list_selection = None;
        self.confirm_delete = None;
    }

    fn open_project_from_path(&mut self, path: &std::path::Path) {
        match load_project(path) {
            Ok(project) => {
                self.project = project;
                self.editor = EditorState::default();
                self.history = History::new();
                self.edit_snapshot_version = None;
                self.status_message = None;
                self.last_saved_version = 0;
                self.rooms_version = 0;
                self.screen = AppScreen::Editor;
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка: {e}"));
            }
        }
    }

    fn save_current_project(&mut self) {
        match save_project(&self.project) {
            Ok(path) => {
                self.last_saved_version = self.history.version;
                self.status_message =
                    Some(format!("Проект сохранён: {}", path.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка сохранения: {e}"));
            }
        }
    }

    fn auto_save(&mut self) {
        if self.history.version != self.last_saved_version {
            if save_project(&self.project).is_ok() {
                self.last_saved_version = self.history.version;
            }
        }
    }

    fn create_new_project(&mut self, name: String, defaults: ProjectDefaults) {
        let mut project = Project::new(name);
        project.defaults = defaults;
        let _ = save_project(&project);
        self.project = project;
        self.editor = EditorState::default();
        self.history = History::new();
        self.edit_snapshot_version = None;
        self.status_message = None;
        self.last_saved_version = 0;
        self.rooms_version = 0;
        self.screen = AppScreen::Editor;
    }

    fn merge_rooms(&mut self, new_rooms: Vec<Room>) {
        use std::collections::HashMap;

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
                new_room.id = old.id;
                new_room.name = old.name.clone();
                preserved_ids.push(old.id);
            }

            merged.push(new_room);
        }

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

    fn delete_selected(&mut self) {
        use crate::editor::Selection;
        match self.editor.selection {
            Selection::Wall(id) => {
                self.history.snapshot(&self.project, "delete wall");
                self.project.remove_wall(id);
            }
            Selection::Opening(id) => {
                self.history.snapshot(&self.project, "delete opening");
                self.project.remove_opening(id);
            }
            Selection::Label(id) => {
                self.history.snapshot(&self.project, "delete label");
                self.project.remove_label(id);
            }
            _ => return,
        }
        self.editor.selection = Selection::None;
    }

    fn set_tool(&mut self, tool: EditorTool) {
        if self.editor.active_tool != tool {
            if self.editor.active_tool == EditorTool::Wall {
                self.editor.wall_tool.reset();
            }
            self.editor.active_tool = tool;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.screen {
            AppScreen::ProjectList => self.show_project_list(ctx),
            AppScreen::Editor => {
                self.handle_keyboard_shortcuts(ctx);
                self.show_toolbar(ctx);
                self.show_left_panel(ctx);
                self.show_right_panel(ctx);
                self.show_price_list_window_ui(ctx);
                self.show_service_picker_window(ctx);
                self.show_project_settings_window(ctx);
                self.show_canvas(ctx);
                self.auto_save();
            }
        }
    }
}
