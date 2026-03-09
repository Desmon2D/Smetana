use eframe::egui;
use glam::DVec2;

use super::draw::DrawCtx;
use super::{App, Selection, Tool, snap, snap_to_grid, snap_to_point};
use crate::model::{Label, Opening, OpeningKind, Point, Project, Room, Wall, distance_to_segment, point_in_polygon};

// ---------------------------------------------------------------------------
// Point preview kind (for draw_tool_preview)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointPreviewKind {
    Existing,
    OnEdge,
    New,
}

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

            // WASD / Arrow camera movement
            if !ui.ctx().wants_keyboard_input() {
                let mut dx = 0.0_f32;
                let mut dy = 0.0_f32;
                ui.input(|i| {
                    let speed = if i.modifiers.shift { 30.0 } else { 15.0 } / self.canvas.zoom;
                    if i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp) { dy += speed; }
                    if i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown) { dy -= speed; }
                    if i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft) { dx += speed; }
                    if i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight) { dx -= speed; }
                });
                if dx != 0.0 || dy != 0.0 {
                    self.canvas.offset.x += dx;
                    self.canvas.offset.y += dy;
                    ctx.request_repaint();
                }
            }

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(45, 45, 48));
            self.canvas.draw_grid(&painter, rect);

            self.canvas.cursor_world_pos = response
                .hover_pos()
                .map(|pos| self.canvas.screen_to_world_dvec2(pos, rect.center()));

            // Compute hover highlight
            self.hover = if let Some(world_pos) = self.canvas.cursor_world_pos {
                let hit = hit_test(world_pos, &self.project, self.canvas.zoom);
                // Don't hover the already-selected element
                if hit == self.selection { Selection::None } else { hit }
            } else {
                Selection::None
            };

            let shift_held = ui.input(|i| i.modifiers.shift);
            match self.active_tool {
                Tool::Select => self.handle_select_tool(ui, &response, rect, space_held, shift_held),
                Tool::Point => {
                    self.handle_point_tool(&response, rect, shift_held, space_held);
                }
                Tool::Edge => self.handle_edge_tool(ui, &response, rect, space_held),
                Tool::Room | Tool::Cutout | Tool::Wall | Tool::Door | Tool::Window => {
                    self.handle_contour_tool(ui, &response, rect, space_held);
                }
                Tool::Label => self.handle_label_tool(&response, rect, space_held),
            }

            // Compute point tool preview position
            let point_preview = if self.active_tool == Tool::Point {
                self.canvas.cursor_world_pos.map(|world_pos| {
                    let snap_result = snap(
                        world_pos,
                        &self.project.points,
                        &self.project.edges,
                        self.canvas.visible_grid_step(),
                        self.canvas.zoom,
                        !shift_held,
                    );
                    let kind = if snap_result.snapped_point.is_some() {
                        PointPreviewKind::Existing
                    } else if snap_result.snapped_edge.is_some() {
                        PointPreviewKind::OnEdge
                    } else {
                        PointPreviewKind::New
                    };
                    (snap_result.position, kind)
                })
            } else {
                None
            };

            // Render (back to front)
            let draw = DrawCtx {
                painter: &painter,
                center: rect.center(),
                canvas: &self.canvas,
                project: &self.project,
                selection: self.selection,
                hover: self.hover,
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
            draw.draw_tool_preview(self.active_tool, &self.tool_state.points, point_preview);

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
        shift_held: bool,
    ) {
        self.handle_select_click(response, rect, space_held);
        self.handle_select_drag(response, rect, space_held, shift_held);
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
        shift_held: bool,
    ) {
        if response.drag_started() && !space_held {
            // Hit-test at drag origin so we drag the point under the cursor,
            // not the previously selected one.
            if let Some(hover) = response.hover_pos() {
                let world_pos = self.canvas.screen_to_world_dvec2(hover, rect.center());
                let hit = hit_test(world_pos, &self.project, self.canvas.zoom);
                if matches!(hit, Selection::Point(_) | Selection::Label(_)) {
                    self.selection = hit;
                    self.history.snapshot(&self.project);
                }
            }
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
                let snapped = if shift_held {
                    cursor_pt
                } else {
                    snap_to_grid(cursor_pt, self.canvas.visible_grid_step())
                };
                if let Some(point) = self.project.point_mut(pid) {
                    point.position = snapped;
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
            &self.project.edges,
            self.canvas.visible_grid_step(),
            self.canvas.zoom,
            !shift_held,
        );

        if let Some(existing_id) = snap_result.snapped_point {
            // Clicked near existing point: just select it.
            self.selection = Selection::Point(existing_id);
        } else if let Some((edge_id, _pa, _pb)) = snap_result.snapped_edge {
            // Clicked on edge: split it and select the new point.
            self.history.snapshot(&self.project);
            let new_id = self.project.split_edge(edge_id, snap_result.position);
            self.selection = Selection::Point(new_id);
            self.edit_snapshot_version = Some(self.history.version);
        } else {
            // Clicked empty space: create free-standing point.
            let point = Point::new(snap_result.position, self.project.defaults.point_height);
            let point_id = point.id;
            self.history.snapshot(&self.project);
            self.project.points.push(point);
            self.selection = Selection::Point(point_id);
            self.edit_snapshot_version = Some(self.history.version);
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
            Tool::Cutout => {
                let positions = self.project.resolve_positions(&points);
                if positions.len() >= 3 {
                    let centroid = DVec2::new(
                        positions.iter().map(|p| p.x).sum::<f64>() / positions.len() as f64,
                        positions.iter().map(|p| p.y).sum::<f64>() / positions.len() as f64,
                    );
                    let room_id = self.project.rooms.iter().find_map(|room| {
                        let room_pts = self.project.resolve_positions(&room.points);
                        if point_in_polygon(centroid, &room_pts) {
                            Some(room.id)
                        } else {
                            None
                        }
                    });
                    if let Some(rid) = room_id
                        && let Some(room) = self.project.room_mut(rid)
                    {
                        room.cutouts.push(points);
                    }
                }
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
                let color = self.project.defaults.wall_color;
                let wall = Wall::new(points, color);
                let wall_id = wall.id;
                self.project.walls.push(wall);
                self.selection = Selection::Wall(wall_id);
            }
            Tool::Door | Tool::Window => {
                let (kind, color) = match self.active_tool {
                    Tool::Door => (
                        OpeningKind::Door {
                            height: self.project.defaults.door_height,
                            width: self.project.defaults.door_width,
                            reveal_width: self.project.defaults.door_reveal_width,
                            swing_edge: 0,
                            swing_outward: true,
                            swing_mirrored: false,
                        },
                        self.project.defaults.door_color,
                    ),
                    _ => (
                        OpeningKind::Window {
                            height: self.project.defaults.window_height,
                            width: self.project.defaults.window_width,
                            sill_height: self.project.defaults.window_sill_height,
                            reveal_width: self.project.defaults.window_reveal_width,
                        },
                        self.project.defaults.window_color,
                    ),
                };
                let opening = Opening::new(points, kind, color);
                let opening_id = opening.id;
                self.project.openings.push(opening);
                self.selection = Selection::Opening(opening_id);
            }
            _ => {}
        }
        self.edit_snapshot_version = Some(self.history.version);
    }

    // ---- Edge tool ----------------------------------------------------------

    fn handle_edge_tool(
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
                self.tool_state.points.push(point_id);
                if self.tool_state.points.len() == 2 {
                    let a = self.tool_state.points[0];
                    let b = self.tool_state.points[1];
                    self.history.snapshot(&self.project);
                    self.project.ensure_edge(a, b);
                    self.tool_state.points.clear();
                    self.edit_snapshot_version = Some(self.history.version);
                }
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.tool_state.points.clear();
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
            self.edit_snapshot_version = Some(self.history.version);
        }
    }
}
