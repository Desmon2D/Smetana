use eframe::egui;
use glam::DVec2;

use super::draw::DrawCtx;
use super::{App, Selection, Tool, snap, snap_to_point};
use crate::model::{Label, Opening, OpeningKind, Point, Project, Room, Wall, distance_to_segment, point_in_polygon};

// ---------------------------------------------------------------------------
// Hit-test (returns Selection directly)
// ---------------------------------------------------------------------------

fn hit_test(world_pos: DVec2, project: &Project, zoom: f32) -> Selection {
    let point_threshold = 10.0 / zoom as f64;
    let edge_threshold = 5.0 / zoom as f64;
    let label_threshold = 20.0 / zoom as f64;

    // 1. Points (highest priority)
    for point in &project.points {
        if point.position.distance(world_pos) < point_threshold {
            return Selection::Point(point.id);
        }
    }

    // 2. Labels
    for label in &project.labels {
        if label.position.distance(world_pos) < label_threshold {
            return Selection::Label(label.id);
        }
    }

    // 3. Edges
    for edge in &project.edges {
        let (Some(a), Some(b)) = (project.point(edge.point_a), project.point(edge.point_b)) else {
            continue;
        };
        if distance_to_segment(world_pos, a.position, b.position) < edge_threshold {
            return Selection::Edge(edge.id);
        }
    }

    // 4. Openings
    for opening in &project.openings {
        if point_in_polygon(world_pos, &project.resolve_positions(&opening.points)) {
            return Selection::Opening(opening.id);
        }
    }

    // 5. Walls
    for wall in &project.walls {
        if point_in_polygon(world_pos, &project.resolve_positions(&wall.points)) {
            return Selection::Wall(wall.id);
        }
    }

    // 6. Rooms (excluding cutouts)
    for room in &project.rooms {
        if point_in_polygon(world_pos, &project.resolve_positions(&room.points)) {
            let in_cutout = room
                .cutouts
                .iter()
                .any(|c| point_in_polygon(world_pos, &project.resolve_positions(c)));
            if !in_cutout {
                return Selection::Room(room.id);
            }
        }
    }

    Selection::None
}

// ---------------------------------------------------------------------------
// App: canvas orchestrator
// ---------------------------------------------------------------------------

impl App {
    pub(super) fn show_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = response.rect;

            let space_held = ui.input(|i| i.key_down(egui::Key::Space));
            self.handle_pan_zoom(&response, ui, rect, space_held);

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(45, 45, 48));
            self.canvas.draw_grid(&painter, rect);

            self.canvas.cursor_world_pos = response
                .hover_pos()
                .map(|pos| self.canvas.screen_to_world_dvec2(pos, rect.center()));

            let shift_held = ui.input(|i| i.modifiers.shift);
            match self.active_tool {
                Tool::Select => self.handle_select_tool(ui, &response, rect, space_held),
                Tool::Point => {
                    self.handle_point_tool(&response, rect, shift_held, space_held);
                }
                Tool::Room | Tool::Wall | Tool::Door | Tool::Window => {
                    self.handle_contour_tool(ui, &response, rect, space_held);
                }
                Tool::Label => self.handle_label_tool(&response, rect, space_held),
            }

            // Render (back to front)
            let draw = DrawCtx {
                painter: &painter,
                center: rect.center(),
                canvas: &self.canvas,
                project: &self.project,
                selection: self.selection,
                visibility: self.visibility,
                label_scale: self.label_scale,
            };

            draw.draw_room_fills();
            draw.draw_wall_fills();
            draw.draw_opening_fills();
            draw.draw_edges();
            draw.draw_points();
            draw.draw_measurement_labels();
            draw.draw_labels();
            draw.draw_tool_preview(self.active_tool, &self.tool_state.points);

            draw.draw_empty_hint(rect, self.active_tool);
            draw.draw_status_bar(rect);
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
            self.canvas.pan(response.drag_delta());
        }

        if space_held && response.dragged_by(egui::PointerButton::Primary) {
            self.canvas.pan(response.drag_delta());
        }

        if response.hovered() {
            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_y != 0.0 {
                let factor = 1.1_f32.powf(scroll_y / 24.0);
                let cursor = response.hover_pos().unwrap_or(rect.center());
                self.canvas.zoom_toward(cursor, rect.center(), factor);
            }
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

        let world_pos = self.canvas.screen_to_world_dvec2(hover, rect.center());
        self.selection = hit_test(world_pos, &self.project, self.canvas.zoom);
    }

    fn handle_select_drag(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if response.drag_started()
            && !space_held
            && matches!(
                self.selection,
                Selection::Point(_) | Selection::Label(_)
            )
        {
            self.history.snapshot(&self.project);
        }

        if !response.dragged_by(egui::PointerButton::Primary) || space_held {
            return;
        }

        let hover = match response.hover_pos() {
            Some(h) => h,
            None => return,
        };
        let cursor_pt = self.canvas.screen_to_world_dvec2(hover, rect.center());

        match self.selection {
            Selection::Point(pid) => {
                if let Some(point) = self.project.point_mut(pid) {
                    point.position = cursor_pt;
                }
            }
            Selection::Label(lid) => {
                if let Some(label) = self.project.label_mut(lid) {
                    label.position = cursor_pt;
                }
            }
            _ => {}
        }
    }

    fn handle_select_keys(&mut self, ui: &egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.selection = Selection::None;
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

        let world_pos = self.canvas.screen_to_world_dvec2(hover, rect.center());
        let snap_result = snap(
            world_pos,
            &self.project.points,
            self.canvas.grid_step,
            self.canvas.zoom,
            !shift_held,
        );

        if let Some(existing_id) = snap_result.snapped_point {
            self.selection = Selection::Point(existing_id);
        } else {
            let point = Point::new(snap_result.position, self.project.defaults.point_height);
            let point_id = point.id;
            self.history.snapshot(&self.project);
            self.project.points.push(point);
            self.selection = Selection::Point(point_id);
        }
    }

    // ---- Contour tools (Room, Wall, Door, Window) ---------------------------

    fn handle_contour_tool(
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
            let world_pos = self.canvas.screen_to_world_dvec2(hover, rect.center());

            if let Some(point_id) =
                snap_to_point(world_pos, &self.project.points, self.canvas.zoom)
            {
                if self.tool_state.points.last() == Some(&point_id) {
                    return;
                }

                if self.tool_state.points.len() >= 3
                    && point_id == self.tool_state.points[0]
                {
                    self.finalize_contour();
                    return;
                }

                self.tool_state.points.push(point_id);
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.finalize_contour();
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.tool_state.points.clear();
            self.tool_state.building_cutout = false;
        }
    }

    fn finalize_contour(&mut self) {
        if self.tool_state.points.len() < 3 {
            return;
        }

        let points = self.tool_state.points.clone();
        self.tool_state.points.clear();

        self.history.snapshot(&self.project);
        self.project.ensure_contour_edges(&points);

        match self.active_tool {
            Tool::Room if self.tool_state.building_cutout => {
                if let Selection::Room(room_id) = self.selection
                    && let Some(room) = self.project.room_mut(room_id)
                {
                    room.cutouts.push(points);
                }
                self.tool_state.building_cutout = false;
            }
            Tool::Room => {
                let room = Room::new(
                    format!("Комната {}", self.project.rooms.len() + 1),
                    points,
                );
                let room_id = room.id;
                self.project.rooms.push(room);
                self.selection = Selection::Room(room_id);
            }
            Tool::Wall => {
                let wall = Wall::new(points);
                let wall_id = wall.id;
                self.project.walls.push(wall);
                self.selection = Selection::Wall(wall_id);
            }
            Tool::Door | Tool::Window => {
                let kind = match self.active_tool {
                    Tool::Door => OpeningKind::Door {
                        height: self.project.defaults.door_height,
                        width: self.project.defaults.door_width,
                    },
                    _ => OpeningKind::Window {
                        height: self.project.defaults.window_height,
                        width: self.project.defaults.window_width,
                        sill_height: self.project.defaults.window_sill_height,
                        reveal_width: self.project.defaults.window_reveal_width,
                    },
                };
                let opening = Opening::new(points, kind);
                let opening_id = opening.id;
                self.project.openings.push(opening);
                self.selection = Selection::Opening(opening_id);
            }
            _ => {}
        }
    }

    // ---- Label tool ---------------------------------------------------------

    fn handle_label_tool(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if response.clicked()
            && !space_held
            && let Some(hover) = response.hover_pos()
        {
            let world_pt = self.canvas.screen_to_world_dvec2(hover, rect.center());
            let label = Label::new("Подпись".to_string(), world_pt);
            let label_id = label.id;
            self.history.snapshot(&self.project);
            self.project.labels.push(label);
            self.selection = Selection::Label(label_id);
        }
    }
}
