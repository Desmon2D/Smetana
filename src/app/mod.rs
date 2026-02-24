use eframe::egui;

use crate::editor::{EditorState, Selection, Tool};
use crate::model::{Project, ProjectDefaults};
use crate::persistence::{ProjectEntry, list_project_entries, load_project, save_project};
use history::History;

mod canvas;
mod canvas_draw;
mod history;
mod project_list;
mod properties_panel;
mod property_edits;
mod toolbar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppScreen {
    ProjectList,
    Editor,
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
    status_message: Option<String>,
    last_saved_version: u64,
    label_scale: f32,
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
            status_message: None,
            last_saved_version: 0,
            label_scale: 1.0,
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
                self.status_message = Some(format!("Проект сохранён: {}", path.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка сохранения: {e}"));
            }
        }
    }

    fn auto_save(&mut self) {
        if self.history.version != self.last_saved_version && save_project(&self.project).is_ok() {
            self.last_saved_version = self.history.version;
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
        self.screen = AppScreen::Editor;
    }

    fn delete_selected(&mut self) {
        match self.editor.selection {
            Selection::Point(id) => {
                self.history.snapshot(&self.project);
                self.project.remove_point(id);
            }
            Selection::Edge(id) => {
                self.history.snapshot(&self.project);
                self.project.edges.retain(|e| e.id != id);
            }
            Selection::Room(id) => {
                self.history.snapshot(&self.project);
                self.project.remove_room(id);
            }
            Selection::Wall(id) => {
                self.history.snapshot(&self.project);
                self.project.remove_wall(id);
            }
            Selection::Opening(id) => {
                self.history.snapshot(&self.project);
                self.project.remove_opening(id);
            }
            Selection::Label(id) => {
                self.history.snapshot(&self.project);
                self.project.remove_label(id);
            }
            Selection::None => return,
        }
        self.editor.selection = Selection::None;
    }

    fn set_tool(&mut self, tool: Tool) {
        if self.editor.active_tool != tool {
            self.editor.tool_state.points.clear();
            self.editor.tool_state.building_cutout = false;
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
                self.show_project_settings_window(ctx);
                self.show_canvas(ctx);
                self.auto_save();
            }
        }
    }
}
