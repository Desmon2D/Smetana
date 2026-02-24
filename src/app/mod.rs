use std::collections::VecDeque;

use eframe::egui;
use glam::DVec2;
use uuid::Uuid;

use crate::model::{Point, Project, ProjectDefaults};
use crate::persistence::{ProjectEntry, list_project_entries, load_project, save_project};

mod canvas;
mod panels;
mod project_list;

// ---------------------------------------------------------------------------
// Tool
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Point,
    Room,
    Wall,
    Door,
    Window,
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
    /// Points collected so far for the contour/polygon.
    points: Vec<Uuid>,
    /// Whether we are building a cutout (Room tool only).
    building_cutout: bool,
}

// ---------------------------------------------------------------------------
// VisibilityMode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibilityMode {
    All,
    Wireframe,
    Rooms,
}

impl VisibilityMode {
    fn show_room_fills(&self) -> bool {
        matches!(self, Self::All | Self::Rooms)
    }

    fn show_wall_fills(&self) -> bool {
        matches!(self, Self::All)
    }

    fn show_opening_fills(&self) -> bool {
        matches!(self, Self::All)
    }
}

// ---------------------------------------------------------------------------
// Canvas (viewport)
// ---------------------------------------------------------------------------

const MIN_ZOOM: f32 = 0.02;
const MAX_ZOOM: f32 = 5.0;

pub struct Canvas {
    /// Pan offset in world coordinates (mm)
    pub offset: egui::Vec2,
    /// Zoom level (pixels per mm)
    pub zoom: f32,
    /// Grid step size in mm
    pub grid_step: f64,
    /// Current cursor position in world coordinates (mm), if hovering over canvas
    pub cursor_world_pos: Option<egui::Pos2>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 0.5,
            grid_step: 100.0,
            cursor_world_pos: None,
        }
    }
}

impl Canvas {
    pub fn world_to_screen(&self, world: egui::Pos2, rect_center: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            (world.x + self.offset.x) * self.zoom + rect_center.x,
            (world.y + self.offset.y) * self.zoom + rect_center.y,
        )
    }

    pub fn screen_to_world(&self, screen: egui::Pos2, rect_center: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            (screen.x - rect_center.x) / self.zoom - self.offset.x,
            (screen.y - rect_center.y) / self.zoom - self.offset.y,
        )
    }

    pub fn screen_to_world_dvec2(
        &self,
        screen: egui::Pos2,
        rect_center: egui::Pos2,
    ) -> DVec2 {
        let p = self.screen_to_world(screen, rect_center);
        DVec2::new(p.x as f64, p.y as f64)
    }

    fn pan(&mut self, screen_delta: egui::Vec2) {
        self.offset += screen_delta / self.zoom;
    }

    fn zoom_toward(&mut self, screen_pos: egui::Pos2, rect_center: egui::Pos2, factor: f32) {
        let world_before = self.screen_to_world(screen_pos, rect_center);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.offset.x = (screen_pos.x - rect_center.x) / self.zoom - world_before.x;
        self.offset.y = (screen_pos.y - rect_center.y) / self.zoom - world_before.y;
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let min_px = 20.0;

        let minor_step = self.grid_step as f32;
        let major_step = minor_step * 10.0;
        let sub_step = minor_step / 10.0;

        let sub_px = sub_step * self.zoom;
        if sub_px >= min_px {
            self.draw_grid_lines(
                painter, rect, center, sub_step,
                egui::Color32::from_rgb(55, 55, 60),
            );
        }

        let minor_px = minor_step * self.zoom;
        if minor_px >= min_px {
            self.draw_grid_lines(
                painter, rect, center, minor_step,
                egui::Color32::from_rgb(65, 65, 72),
            );
        }

        let major_px = major_step * self.zoom;
        if major_px >= min_px {
            self.draw_grid_lines(
                painter, rect, center, major_step,
                egui::Color32::from_rgb(80, 80, 88),
            );
        }

        // Origin axes
        let origin = self.world_to_screen(egui::Pos2::ZERO, center);
        let axis_color = egui::Color32::from_rgb(100, 100, 115);

        if origin.x >= rect.left() && origin.x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(origin.x, rect.top()),
                    egui::pos2(origin.x, rect.bottom()),
                ],
                egui::Stroke::new(1.5, axis_color),
            );
        }
        if origin.y >= rect.top() && origin.y <= rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), origin.y),
                    egui::pos2(rect.right(), origin.y),
                ],
                egui::Stroke::new(1.5, axis_color),
            );
        }
    }

    fn draw_grid_lines(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        center: egui::Pos2,
        step: f32,
        color: egui::Color32,
    ) {
        let tl = self.screen_to_world(rect.left_top(), center);
        let br = self.screen_to_world(rect.right_bottom(), center);

        let x0 = (tl.x / step).floor() as i64;
        let x1 = (br.x / step).ceil() as i64;
        let y0 = (tl.y / step).floor() as i64;
        let y1 = (br.y / step).ceil() as i64;

        let stroke = egui::Stroke::new(1.0, color);

        for i in x0..=x1 {
            let sx = self
                .world_to_screen(egui::pos2(i as f32 * step, 0.0), center)
                .x;
            painter.line_segment(
                [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                stroke,
            );
        }

        for i in y0..=y1 {
            let sy = self
                .world_to_screen(egui::pos2(0.0, i as f32 * step), center)
                .y;
            painter.line_segment(
                [egui::pos2(rect.left(), sy), egui::pos2(rect.right(), sy)],
                stroke,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Snap
// ---------------------------------------------------------------------------

const POINT_SNAP_RADIUS: f64 = 15.0;

struct SnapResult {
    position: DVec2,
    snapped_point: Option<Uuid>,
}

fn snap_to_point(world_pos: DVec2, points: &[Point], zoom: f32) -> Option<Uuid> {
    let threshold = POINT_SNAP_RADIUS / zoom as f64;
    let mut best: Option<(Uuid, f64)> = None;
    for p in points {
        let dist = p.position.distance(world_pos);
        if dist < threshold && (best.is_none() || dist < best.unwrap().1) {
            best = Some((p.id, dist));
        }
    }
    best.map(|(id, _)| id)
}

fn snap_to_grid(world_pos: DVec2, grid_step: f64) -> DVec2 {
    DVec2::new(
        (world_pos.x / grid_step).round() * grid_step,
        (world_pos.y / grid_step).round() * grid_step,
    )
}

fn snap(
    world_pos: DVec2,
    points: &[Point],
    grid_step: f64,
    zoom: f32,
    snap_enabled: bool,
) -> SnapResult {
    if !snap_enabled {
        return SnapResult {
            position: world_pos,
            snapped_point: None,
        };
    }

    if let Some(id) = snap_to_point(world_pos, points, zoom) {
        let pos = points.iter().find(|p| p.id == id).unwrap().position;
        return SnapResult {
            position: pos,
            snapped_point: Some(id),
        };
    }

    let grid_pos = snap_to_grid(world_pos, grid_step);
    SnapResult {
        position: grid_pos,
        snapped_point: None,
    }
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
    // Editor fields (formerly EditorState)
    active_tool: Tool,
    selection: Selection,
    canvas: Canvas,
    tool_state: ToolState,
    visibility: VisibilityMode,

    history: History,
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
            active_tool: Tool::Select,
            selection: Selection::None,
            canvas: Canvas::default(),
            tool_state: ToolState::default(),
            visibility: VisibilityMode::All,

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
                self.active_tool = Tool::Select;
                self.selection = Selection::None;
                self.canvas = Canvas::default();
                self.tool_state = ToolState::default();
                self.visibility = VisibilityMode::All;
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
        self.active_tool = Tool::Select;
        self.selection = Selection::None;
        self.canvas = Canvas::default();
        self.tool_state = ToolState::default();
        self.visibility = VisibilityMode::All;
        self.history = History::new();
        self.edit_snapshot_version = None;
        self.status_message = None;
        self.last_saved_version = 0;
        self.screen = AppScreen::Editor;
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
            Selection::Point(_) => self.project.remove_point(id),
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
            self.tool_state.building_cutout = false;
            self.active_tool = tool;
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
