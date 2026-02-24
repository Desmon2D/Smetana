use eframe::egui;
use glam::DVec2;
use uuid::Uuid;

use super::App;
use crate::editor::{Selection, Tool, snap, snap_to_point};
use crate::model::{
    Label, Opening, OpeningKind, Point, Room, Wall, distance_to_segment, point_in_polygon,
};

// ---------------------------------------------------------------------------
// Hit-test
// ---------------------------------------------------------------------------

enum HitResult {
    Point(Uuid),
    Label(Uuid),
    Edge(Uuid),
    Opening(Uuid),
    Wall(Uuid),
    Room(Uuid),
    Nothing,
}

/// Hit-test in world space. Priority (front to back):
/// Points > Labels > Edges > Openings > Walls > Rooms.
fn hit_test(world_pos: DVec2, project: &crate::model::Project, zoom: f32) -> HitResult {
    let point_threshold = 10.0 / zoom as f64;
    let edge_threshold = 5.0 / zoom as f64;
    let label_threshold = 20.0 / zoom as f64;

    // 1. Points (highest priority — always on top)
    for point in &project.points {
        if point.position.distance(world_pos) < point_threshold {
            return HitResult::Point(point.id);
        }
    }

    // 2. Labels (UI overlays, high priority)
    for label in &project.labels {
        if label.position.distance(world_pos) < label_threshold {
            return HitResult::Label(label.id);
        }
    }

    // 3. Edges (line segments)
    for edge in &project.edges {
        let (Some(a), Some(b)) = (project.point(edge.point_a), project.point(edge.point_b)) else {
            continue;
        };
        if distance_to_segment(world_pos, a.position, b.position) < edge_threshold {
            return HitResult::Edge(edge.id);
        }
    }

    // 4. Openings (polygon)
    for opening in &project.openings {
        let polygon: Vec<DVec2> = opening
            .points
            .iter()
            .filter_map(|id| project.point(*id))
            .map(|p| p.position)
            .collect();
        if point_in_polygon(world_pos, &polygon) {
            return HitResult::Opening(opening.id);
        }
    }

    // 5. Walls (polygon)
    for wall in &project.walls {
        let polygon: Vec<DVec2> = wall
            .points
            .iter()
            .filter_map(|id| project.point(*id))
            .map(|p| p.position)
            .collect();
        if point_in_polygon(world_pos, &polygon) {
            return HitResult::Wall(wall.id);
        }
    }

    // 6. Rooms (polygon, excluding cutouts)
    for room in &project.rooms {
        let polygon: Vec<DVec2> = room
            .points
            .iter()
            .filter_map(|id| project.point(*id))
            .map(|p| p.position)
            .collect();
        if point_in_polygon(world_pos, &polygon) {
            let in_cutout = room.cutouts.iter().any(|c| {
                let cutout_poly: Vec<DVec2> = c
                    .iter()
                    .filter_map(|id| project.point(*id))
                    .map(|p| p.position)
                    .collect();
                point_in_polygon(world_pos, &cutout_poly)
            });
            if !in_cutout {
                return HitResult::Room(room.id);
            }
        }
    }

    HitResult::Nothing
}

// ---------------------------------------------------------------------------
// App methods
// ---------------------------------------------------------------------------

impl App {
    // ---- show_canvas orchestrator -------------------------------------------

    pub(super) fn show_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = response.rect;

            let space_held = ui.input(|i| i.key_down(egui::Key::Space));
            self.handle_pan_zoom(&response, ui, rect, space_held);

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(45, 45, 48));
            self.editor.canvas.draw_grid(&painter, rect);

            if let Some(pos) = response.hover_pos() {
                let world = self.editor.canvas.screen_to_world(pos, rect.center());
                self.editor.canvas.cursor_world_pos = Some(world);
            } else {
                self.editor.canvas.cursor_world_pos = None;
            }

            let shift_held = ui.input(|i| i.modifiers.shift);
            match self.editor.active_tool {
                Tool::Select => self.handle_select_tool(ui, &response, rect, space_held),
                Tool::Point => {
                    self.handle_point_tool(&response, rect, shift_held, space_held);
                }
                Tool::Room => self.handle_room_tool(ui, &response, rect, space_held),
                Tool::Wall | Tool::Door | Tool::Window => {
                    self.handle_polygon_tool(ui, &response, rect, space_held);
                }
                Tool::Label => self.handle_label_tool(&response, rect, space_held),
            }

            // Render (back to front)
            self.draw_room_fills(&painter, rect);
            self.draw_wall_fills(&painter, rect);
            self.draw_opening_fills(&painter, rect);
            self.draw_edges(&painter, rect);
            self.draw_points(&painter, rect);
            self.draw_measurement_labels(&painter, rect);
            self.draw_labels(&painter, rect);
            self.draw_tool_preview(&painter, rect);

            self.draw_empty_hint(&painter, rect);
            self.draw_status_bar(&painter, rect);
        });
    }

    // ---- Pan/zoom -----------------------------------------------------------

    fn handle_pan_zoom(
        &mut self,
        response: &egui::Response,
        ui: &egui::Ui,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if response.dragged_by(egui::PointerButton::Middle) {
            self.editor.canvas.pan(response.drag_delta());
        }

        if space_held && response.dragged_by(egui::PointerButton::Primary) {
            self.editor.canvas.pan(response.drag_delta());
        }

        if response.hovered() {
            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_y != 0.0 {
                let factor = 1.1_f32.powf(scroll_y / 24.0);
                let cursor = response.hover_pos().unwrap_or(rect.center());
                self.editor
                    .canvas
                    .zoom_toward(cursor, rect.center(), factor);
            }
        }
    }

    // ---- Status bar ---------------------------------------------------------

    fn draw_status_bar(&self, painter: &egui::Painter, rect: egui::Rect) {
        if let Some(pos) = self.editor.canvas.cursor_world_pos {
            let zoom_pct = self.editor.canvas.zoom * 200.0;
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
    }

    // ---- Empty-canvas hint --------------------------------------------------

    fn draw_empty_hint(&self, painter: &egui::Painter, rect: egui::Rect) {
        if self.project.points.is_empty() {
            let tool_hint = match self.editor.active_tool {
                Tool::Select => "Режим выбора — кликните на объект",
                Tool::Point => "Кликните для размещения точки",
                Tool::Room => "Сначала создайте точки (P), затем соберите контур",
                Tool::Wall => "Сначала создайте точки (P), затем соберите полигон",
                Tool::Door => "Сначала создайте точки (P), затем полигон двери",
                Tool::Window => "Сначала создайте точки (P), затем полигон окна",
                Tool::Label => "Кликните для размещения надписи",
            };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tool_hint,
                egui::FontId::proportional(16.0),
                egui::Color32::from_rgb(120, 120, 120),
            );
        }
    }

    // ---- Select tool --------------------------------------------------------

    fn handle_select_tool(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        self.handle_select_click(response, rect, space_held);
        self.handle_select_drag(response, rect, space_held);
        self.handle_select_keys(ui);
    }

    fn handle_select_click(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if !response.clicked() || space_held {
            return;
        }
        self.edit_snapshot_version = None;

        let hover = match response.hover_pos() {
            Some(h) => h,
            None => return,
        };

        let world_pos = self
            .editor
            .canvas
            .screen_to_world_dvec2(hover, rect.center());

        self.editor.selection = match hit_test(world_pos, &self.project, self.editor.canvas.zoom) {
            HitResult::Point(id) => Selection::Point(id),
            HitResult::Label(id) => Selection::Label(id),
            HitResult::Edge(id) => Selection::Edge(id),
            HitResult::Opening(id) => Selection::Opening(id),
            HitResult::Wall(id) => Selection::Wall(id),
            HitResult::Room(id) => Selection::Room(id),
            HitResult::Nothing => Selection::None,
        };
    }

    fn handle_select_drag(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        // Snapshot on drag start for draggable objects
        if response.drag_started()
            && !space_held
            && matches!(
                self.editor.selection,
                Selection::Point(_) | Selection::Label(_)
            )
        {
            self.history.snapshot(&self.project, "drag");
        }

        if !response.dragged_by(egui::PointerButton::Primary) || space_held {
            return;
        }

        let hover = match response.hover_pos() {
            Some(h) => h,
            None => return,
        };
        let cursor_pt = self
            .editor
            .canvas
            .screen_to_world_dvec2(hover, rect.center());

        match self.editor.selection {
            Selection::Point(pid) => {
                if let Some(point) = self.project.point_mut(pid) {
                    point.position = cursor_pt;
                }
            }
            Selection::Label(lid) => {
                if let Some(label) = self.project.labels.iter_mut().find(|l| l.id == lid) {
                    label.position = cursor_pt;
                }
            }
            _ => {}
        }
    }

    fn handle_select_keys(&mut self, ui: &egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor.selection = Selection::None;
        }

        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            self.delete_selected();
        }
    }

    // ---- Point tool ---------------------------------------------------------

    fn handle_point_tool(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        shift_held: bool,
        space_held: bool,
    ) {
        if !response.clicked() || space_held {
            return;
        }

        let hover = match response.hover_pos() {
            Some(h) => h,
            None => return,
        };

        let world_pos = self
            .editor
            .canvas
            .screen_to_world_dvec2(hover, rect.center());
        let snap_result = snap(
            world_pos,
            &self.project.points,
            self.editor.canvas.grid_step,
            self.editor.canvas.zoom,
            !shift_held, // snap disabled when Shift held
        );

        if let Some(existing_id) = snap_result.snapped_point {
            // Snapped to existing point — just select it
            self.editor.selection = Selection::Point(existing_id);
        } else {
            // Create new point at snapped position
            let point = Point::new(snap_result.position, self.project.defaults.point_height);
            let point_id = point.id;
            self.history.snapshot(&self.project, "add point");
            self.project.points.push(point);
            self.editor.selection = Selection::Point(point_id);
        }
    }

    // ---- Room tool ----------------------------------------------------------

    fn handle_room_tool(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if response.clicked()
            && !space_held
            && let Some(hover) = response.hover_pos()
        {
            let world_pos = self
                .editor
                .canvas
                .screen_to_world_dvec2(hover, rect.center());

            // Must click on existing point
            if let Some(point_id) =
                snap_to_point(world_pos, &self.project.points, self.editor.canvas.zoom)
            {
                // Avoid duplicate consecutive points
                if self.editor.room_tool.points.last() == Some(&point_id) {
                    return;
                }

                // If clicking first point and enough collected, close the contour
                if self.editor.room_tool.points.len() >= 3
                    && point_id == self.editor.room_tool.points[0]
                {
                    self.finalize_room();
                    return;
                }

                self.editor.room_tool.points.push(point_id);
            }
        }

        // Enter to finalize
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.finalize_room();
        }

        // Escape to cancel
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor.room_tool.points.clear();
            self.editor.room_tool.building_cutout = false;
        }
    }

    fn finalize_room(&mut self) {
        if self.editor.room_tool.points.len() < 3 {
            return;
        }

        let points = self.editor.room_tool.points.clone();

        if self.editor.room_tool.building_cutout {
            // Adding a cutout to the selected room
            if let Selection::Room(room_id) = self.editor.selection {
                self.history.snapshot(&self.project, "add cutout");
                self.project.ensure_contour_edges(&points);
                if let Some(room) = self.project.rooms.iter_mut().find(|r| r.id == room_id) {
                    room.cutouts.push(points);
                }
            }
            self.editor.room_tool.points.clear();
            self.editor.room_tool.building_cutout = false;
        } else {
            // Creating a new room
            self.history.snapshot(&self.project, "add room");

            let room = Room::new(
                format!("Комната {}", self.project.rooms.len() + 1),
                points.clone(),
            );

            self.project.ensure_contour_edges(&points);
            let room_id = room.id;
            self.project.rooms.push(room);

            self.editor.room_tool.points.clear();
            self.editor.room_tool.building_cutout = false;
            self.editor.selection = Selection::Room(room_id);
        }
    }

    // ---- Polygon tools (Wall, Door, Window) ---------------------------------

    fn handle_polygon_tool(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if response.clicked()
            && !space_held
            && let Some(hover) = response.hover_pos()
        {
            let world_pos = self
                .editor
                .canvas
                .screen_to_world_dvec2(hover, rect.center());

            // Must click on existing point
            if let Some(point_id) =
                snap_to_point(world_pos, &self.project.points, self.editor.canvas.zoom)
            {
                // Avoid duplicate consecutive
                if self.editor.polygon_tool.points.last() == Some(&point_id) {
                    return;
                }

                // Close if clicking first point and enough collected
                if self.editor.polygon_tool.points.len() >= 3
                    && point_id == self.editor.polygon_tool.points[0]
                {
                    self.finalize_polygon();
                    return;
                }

                self.editor.polygon_tool.points.push(point_id);
            }
        }

        // Enter to finalize
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.finalize_polygon();
        }

        // Escape to cancel
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor.polygon_tool.points.clear();
        }
    }

    fn finalize_polygon(&mut self) {
        if self.editor.polygon_tool.points.len() < 3 {
            return;
        }

        let points = self.editor.polygon_tool.points.clone();
        self.editor.polygon_tool.points.clear();

        match self.editor.active_tool {
            Tool::Wall => {
                self.history.snapshot(&self.project, "add wall");
                let wall = Wall::new(points.clone());
                let wall_id = wall.id;
                self.project.ensure_contour_edges(&points);
                self.project.walls.push(wall);
                self.editor.selection = Selection::Wall(wall_id);
            }
            Tool::Door => {
                self.history.snapshot(&self.project, "add door");
                let kind = OpeningKind::Door {
                    height: self.project.defaults.door_height,
                    width: self.project.defaults.door_width,
                };
                let opening = Opening::new(points.clone(), kind);
                let opening_id = opening.id;
                self.project.ensure_contour_edges(&points);
                self.project.openings.push(opening);
                self.editor.selection = Selection::Opening(opening_id);
            }
            Tool::Window => {
                self.history.snapshot(&self.project, "add window");
                let kind = OpeningKind::Window {
                    height: self.project.defaults.window_height,
                    width: self.project.defaults.window_width,
                    sill_height: self.project.defaults.window_sill_height,
                    reveal_width: self.project.defaults.window_reveal_width,
                };
                let opening = Opening::new(points.clone(), kind);
                let opening_id = opening.id;
                self.project.ensure_contour_edges(&points);
                self.project.openings.push(opening);
                self.editor.selection = Selection::Opening(opening_id);
            }
            _ => {}
        }
    }

    // ---- Label tool ---------------------------------------------------------

    fn handle_label_tool(&mut self, response: &egui::Response, rect: egui::Rect, space_held: bool) {
        if response.clicked()
            && !space_held
            && let Some(hover) = response.hover_pos()
        {
            let world_pt = self
                .editor
                .canvas
                .screen_to_world_dvec2(hover, rect.center());
            let label = Label::new("Подпись".to_string(), world_pt);
            let label_id = label.id;
            self.history.snapshot(&self.project, "add label");
            self.project.labels.push(label);
            self.editor.selection = Selection::Label(label_id);
        }
    }
}
