use std::collections::VecDeque;
use std::time::Instant;

use eframe::egui;
use uuid::Uuid;

use crate::model::{Project, ProjectDefaults};
use crate::persistence::{ProjectEntry, list_project_entries, load_project, save_project};

mod canvas;
mod draw;
mod panels;
mod project_list;
pub mod viewport;

pub use viewport::{Canvas, VisibilityMode, snap, snap_to_grid, snap_to_point};

// ---------------------------------------------------------------------------
// Tool
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Point,
    Edge,
    Cutout,
    Room,
    Door,
    Window,
    Wall,
    Label,
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    None,
    Point(Uuid),
    Edge(Uuid),
    Opening(Uuid),
    Wall(Uuid),
    Room(Uuid),
    Label(Uuid),
}

impl Selection {
    fn id(&self) -> Option<Uuid> {
        match self {
            Self::None => None,
            Self::Point(id)
            | Self::Edge(id)
            | Self::Opening(id)
            | Self::Wall(id)
            | Self::Room(id)
            | Self::Label(id) => Some(*id),
        }
    }
}

// ---------------------------------------------------------------------------
// ToolState
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ToolState {
    /// Points collected so far for the contour/polygon or edge tool.
    points: Vec<Uuid>,
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

struct History {
    undo_stack: VecDeque<Project>,
    redo_stack: VecDeque<Project>,
    /// Monotonically increasing counter, bumped on every snapshot/undo/redo.
    version: u64,
    max_entries: usize,
}

impl History {
    fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            version: 0,
            max_entries: 100,
        }
    }

    /// Save current project state before a mutation.
    fn snapshot(&mut self, project: &Project) {
        self.undo_stack.push_back(project.clone());
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
        self.version += 1;
    }

    fn undo(&mut self, project: &mut Project) -> bool {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(project.clone());
            *project = prev;
            self.version += 1;
            true
        } else {
            false
        }
    }

    fn redo(&mut self, project: &mut Project) -> bool {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(project.clone());
            *project = next;
            self.version += 1;
            true
        } else {
            false
        }
    }

    fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Bump version without storing a snapshot. For non-undoable state changes.
    fn mark_dirty(&mut self) {
        self.version += 1;
    }
}

// ---------------------------------------------------------------------------
// AppScreen
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppScreen {
    ProjectList,
    Editor,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    screen: AppScreen,
    project_entries: Vec<ProjectEntry>,
    project_list_selection: Option<usize>,
    new_project_name: String,
    confirm_delete: Option<usize>,
    show_new_project_dialog: bool,
    new_project_defaults: ProjectDefaults,
    show_project_settings: bool,

    project: Project,
    active_tool: Tool,
    selection: Selection,
    canvas: Canvas,
    tool_state: ToolState,
    visibility: VisibilityMode,

    hover: Selection,
    history: History,
    edit_snapshot_version: Option<u64>,
    status_message: Option<(String, Instant)>,
    last_saved_version: u64,
    last_save_time: Instant,
    label_scale: f32,
    copied_color: Option<[u8; 4]>,
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
            active_tool: Tool::Select,
            selection: Selection::None,
            canvas: Canvas::default(),
            tool_state: ToolState::default(),
            visibility: VisibilityMode::All,

            hover: Selection::None,
            history: History::new(),
            edit_snapshot_version: None,
            status_message: None,
            last_saved_version: 0,
            last_save_time: Instant::now(),
            label_scale: 1.0,
            copied_color: None,
        }
    }

    /// Reset all editor state and switch to the editor screen.
    fn reset_editor(&mut self, project: Project) {
        self.project = project;
        self.active_tool = Tool::Select;
        self.selection = Selection::None;
        self.canvas = Canvas::default();
        self.tool_state = ToolState::default();
        self.visibility = VisibilityMode::All;
        self.hover = Selection::None;
        self.history = History::new();
        self.edit_snapshot_version = None;
        self.status_message = None;
        self.last_saved_version = 0;
        self.last_save_time = Instant::now();
        self.screen = AppScreen::Editor;
    }

    fn refresh_project_list(&mut self) {
        self.project_entries = list_project_entries().unwrap_or_default();
        self.project_list_selection = None;
        self.confirm_delete = None;
    }

    fn open_project_from_path(&mut self, path: &std::path::Path) {
        match load_project(path) {
            Ok(project) => self.reset_editor(project),
            Err(e) => {
                self.status_message = Some((format!("Ошибка: {e}"), Instant::now()));
            }
        }
    }

    fn save_current_project(&mut self) {
        match save_project(&self.project) {
            Ok(path) => {
                self.last_saved_version = self.history.version;
                self.status_message =
                    Some((format!("Проект сохранён: {}", path.display()), Instant::now()));
            }
            Err(e) => {
                self.status_message = Some((format!("Ошибка сохранения: {e}"), Instant::now()));
            }
        }
    }

    fn auto_save(&mut self) {
        if self.history.version != self.last_saved_version
            && self.last_save_time.elapsed().as_secs() >= 2
            && save_project(&self.project).is_ok()
        {
            self.last_saved_version = self.history.version;
            self.last_save_time = Instant::now();
        }
        if self
            .status_message
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs() >= 5)
        {
            self.status_message = None;
        }
    }

    fn create_new_project(&mut self, name: String, defaults: ProjectDefaults) {
        let mut project = Project::new(name);
        project.defaults = defaults;
        let _ = save_project(&project);
        self.reset_editor(project);
    }

    fn close_new_project_form(&mut self) {
        self.new_project_name.clear();
        self.new_project_defaults = ProjectDefaults::default();
        self.show_new_project_dialog = false;
    }

    fn ensure_edit_snapshot(&mut self) {
        if self.edit_snapshot_version != Some(self.history.version) {
            self.history.snapshot(&self.project);
            self.edit_snapshot_version = Some(self.history.version);
        }
    }

    fn delete_selected(&mut self) {
        let Some(id) = self.selection.id() else {
            return;
        };
        self.history.snapshot(&self.project);
        match self.selection {
            Selection::Point(_) => self.project.smart_remove_point(id),
            Selection::Edge(_) => self.project.remove_edge(id),
            Selection::Room(_) => self.project.remove_room(id),
            Selection::Wall(_) => self.project.remove_wall(id),
            Selection::Opening(_) => self.project.remove_opening(id),
            Selection::Label(_) => self.project.remove_label(id),
            Selection::None => unreachable!(),
        }
        self.selection = Selection::None;
    }

    fn set_tool(&mut self, tool: Tool) {
        if self.active_tool != tool {
            self.tool_state.points.clear();
            self.active_tool = tool;
        }
    }

    /// Clear selection if the referenced entity no longer exists in the project.
    fn validate_selection(&mut self) {
        let valid = match self.selection {
            Selection::None => true,
            Selection::Point(id) => self.project.point(id).is_some(),
            Selection::Edge(id) => self.project.edge(id).is_some(),
            Selection::Room(id) => self.project.room(id).is_some(),
            Selection::Wall(id) => self.project.wall(id).is_some(),
            Selection::Opening(id) => self.project.opening(id).is_some(),
            Selection::Label(id) => self.project.label(id).is_some(),
        };
        if !valid {
            self.selection = Selection::None;
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
