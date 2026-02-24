use eframe::egui;
use glam::DVec2;

use super::{App, Canvas, Selection, Tool, snap, snap_to_point};
use crate::model::{Label, Opening, OpeningKind, Point, Project, Room, Wall, distance_to_segment, point_in_polygon};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ROOM_COLORS: &[(u8, u8, u8)] = &[
    (70, 130, 180),
    (60, 179, 113),
    (218, 165, 32),
    (178, 102, 178),
    (205, 92, 92),
    (72, 209, 204),
    (244, 164, 96),
    (123, 104, 238),
];

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
// Coordinate helpers
// ---------------------------------------------------------------------------

fn world_to_screen(canvas: &Canvas, center: egui::Pos2, p: DVec2) -> egui::Pos2 {
    canvas.world_to_screen(egui::pos2(p.x as f32, p.y as f32), center)
}

fn polygon_screen_coords(
    point_ids: &[uuid::Uuid],
    project: &Project,
    canvas: &Canvas,
    center: egui::Pos2,
) -> Vec<egui::Pos2> {
    project
        .resolve_positions(point_ids)
        .iter()
        .map(|&pos| world_to_screen(canvas, center, pos))
        .collect()
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

fn paint_rotated_text(
    painter: &egui::Painter,
    center_pos: egui::Pos2,
    text: String,
    font_id: egui::FontId,
    color: egui::Color32,
    angle_rad: f32,
) {
    let angle = if angle_rad > std::f32::consts::FRAC_PI_2 {
        angle_rad - std::f32::consts::PI
    } else if angle_rad < -std::f32::consts::FRAC_PI_2 {
        angle_rad + std::f32::consts::PI
    } else {
        angle_rad
    };

    let galley = painter.layout_no_wrap(text, font_id, color);
    let w = galley.size().x;
    let h = galley.size().y;

    let (sin_a, cos_a) = angle.sin_cos();
    let offset_x = cos_a * (w / 2.0) - sin_a * (h / 2.0);
    let offset_y = sin_a * (w / 2.0) + cos_a * (h / 2.0);
    let pos = egui::pos2(center_pos.x - offset_x, center_pos.y - offset_y);

    let text_shape = egui::epaint::TextShape::new(pos, galley, color).with_angle(angle);
    painter.add(text_shape);
}

// ---------------------------------------------------------------------------
// Triangulation helpers
// ---------------------------------------------------------------------------

fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]> {
    if vertices.len() < 3 {
        return Vec::new();
    }
    let coords: Vec<f32> = vertices.iter().flat_map(|p| [p.x, p.y]).collect();
    let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
    indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()
}

/// Triangulate and fill a simple polygon (no holes).
fn fill_polygon(painter: &egui::Painter, screen_pts: &[egui::Pos2], fill: egui::Color32) {
    for tri in &triangulate(screen_pts) {
        painter.add(egui::Shape::convex_polygon(
            vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]],
            fill,
            egui::Stroke::NONE,
        ));
    }
}

// ---------------------------------------------------------------------------
// Door / Window symbol rendering
// ---------------------------------------------------------------------------

fn draw_door_symbol(
    painter: &egui::Painter,
    p_left: egui::Pos2,
    p_right: egui::Pos2,
    is_selected: bool,
) {
    let color = if is_selected {
        egui::Color32::from_rgb(240, 180, 80)
    } else {
        egui::Color32::from_rgb(180, 120, 60)
    };
    let stroke_w = if is_selected { 2.0 } else { 1.5 };

    painter.line_segment([p_left, p_right], egui::Stroke::new(stroke_w, color));

    let arc_r = ((p_right.x - p_left.x).powi(2) + (p_right.y - p_left.y).powi(2)).sqrt();
    if arc_r > 1.0 {
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
            pts.push(egui::pos2(p_left.x + d_x * arc_r, p_left.y + d_y * arc_r));
        }
        for i in 0..n_seg {
            painter.line_segment([pts[i], pts[i + 1]], egui::Stroke::new(stroke_w, color));
        }
    }
}

fn draw_window_symbol(
    painter: &egui::Painter,
    p_left: egui::Pos2,
    p_right: egui::Pos2,
    nx: f32,
    ny: f32,
    is_selected: bool,
) {
    let color = if is_selected {
        egui::Color32::from_rgb(120, 210, 255)
    } else {
        egui::Color32::from_rgb(80, 160, 220)
    };
    let stroke_w = if is_selected { 2.0 } else { 1.5 };

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

            if let Some(pos) = response.hover_pos() {
                let world = self.canvas.screen_to_world(pos, rect.center());
                self.canvas.cursor_world_pos = Some(world);
            } else {
                self.canvas.cursor_world_pos = None;
            }

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

    // ---- Status bar ---------------------------------------------------------

    fn draw_status_bar(&self, painter: &egui::Painter, rect: egui::Rect) {
        if let Some(pos) = self.canvas.cursor_world_pos {
            let zoom_pct = self.canvas.zoom * 200.0;
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
            let tool_hint = match self.active_tool {
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
                if let Some(label) = self.project.labels.iter_mut().find(|l| l.id == lid) {
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
                    && let Some(room) = self.project.rooms.iter_mut().find(|r| r.id == room_id)
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

    // -----------------------------------------------------------------------
    // Drawing: Room fills
    // -----------------------------------------------------------------------

    fn draw_room_fills(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.visibility.show_room_fills() {
            return;
        }

        let center = rect.center();

        for (i, room) in self.project.rooms.iter().enumerate() {
            let screen_pts =
                polygon_screen_coords(&room.points, &self.project, &self.canvas, center);
            if screen_pts.len() < 3 {
                continue;
            }

            let (r, g, b) = ROOM_COLORS[i % ROOM_COLORS.len()];
            let is_selected = self.selection == Selection::Room(room.id);
            let alpha = if is_selected { 60 } else { 40 };
            let fill = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);

            if room.cutouts.is_empty() {
                fill_polygon(painter, &screen_pts, fill);
            } else {
                // Triangulation with holes
                let mut coords: Vec<f32> = screen_pts.iter().flat_map(|p| [p.x, p.y]).collect();
                let mut hole_indices = Vec::new();

                for cutout in &room.cutouts {
                    let cutout_pts =
                        polygon_screen_coords(cutout, &self.project, &self.canvas, center);
                    if cutout_pts.len() < 3 {
                        continue;
                    }
                    hole_indices.push(coords.len() / 2);
                    for p in cutout_pts.iter().rev() {
                        coords.extend([p.x, p.y]);
                    }
                }

                let all_pts: Vec<egui::Pos2> =
                    coords.chunks(2).map(|c| egui::pos2(c[0], c[1])).collect();
                let indices = earcutr::earcut(&coords, &hole_indices, 2).unwrap_or_default();
                for tri in indices.chunks(3) {
                    if tri.len() == 3 {
                        let tri_pts = vec![all_pts[tri[0]], all_pts[tri[1]], all_pts[tri[2]]];
                        painter.add(egui::Shape::convex_polygon(
                            tri_pts,
                            fill,
                            egui::Stroke::NONE,
                        ));
                    }
                }
            }

            // Room outline
            let outline_color = egui::Color32::from_rgba_unmultiplied(r, g, b, 80);
            let stroke_w = if is_selected { 2.0 } else { 1.0 };
            painter.add(egui::Shape::closed_line(
                screen_pts,
                egui::Stroke::new(stroke_w, outline_color),
            ));
        }
    }

    // -----------------------------------------------------------------------
    // Drawing: Wall fills
    // -----------------------------------------------------------------------

    fn draw_wall_fills(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.visibility.show_wall_fills() {
            return;
        }

        let center = rect.center();
        let wall_outline = egui::Color32::from_rgb(40, 40, 42);

        for wall in &self.project.walls {
            let screen_pts =
                polygon_screen_coords(&wall.points, &self.project, &self.canvas, center);
            if screen_pts.len() < 3 {
                continue;
            }

            let is_selected = self.selection == Selection::Wall(wall.id);
            let [r, g, b, a] = wall.color;
            let fill = if is_selected {
                egui::Color32::from_rgba_unmultiplied(
                    r.saturating_add(40),
                    g.saturating_add(40),
                    b.saturating_add(40),
                    a,
                )
            } else {
                egui::Color32::from_rgba_unmultiplied(r, g, b, a)
            };

            fill_polygon(painter, &screen_pts, fill);

            let outline_stroke = if is_selected {
                egui::Stroke::new(2.5, egui::Color32::from_rgb(60, 160, 255))
            } else {
                egui::Stroke::new(1.0, wall_outline)
            };
            painter.add(egui::Shape::closed_line(screen_pts, outline_stroke));
        }
    }

    // -----------------------------------------------------------------------
    // Drawing: Opening fills
    // -----------------------------------------------------------------------

    fn draw_opening_fills(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.visibility.show_opening_fills() {
            return;
        }

        let center = rect.center();

        for opening in &self.project.openings {
            let screen_pts =
                polygon_screen_coords(&opening.points, &self.project, &self.canvas, center);
            if screen_pts.len() < 2 {
                continue;
            }

            let is_selected = self.selection == Selection::Opening(opening.id);

            // Draw the opening gap (background cutout)
            if screen_pts.len() >= 3 {
                fill_polygon(painter, &screen_pts, egui::Color32::from_rgb(45, 45, 48));
            }

            // Draw the symbol along the first edge
            let p_left = screen_pts[0];
            let p_right = screen_pts[1];

            let dx = p_right.x - p_left.x;
            let dy = p_right.y - p_left.y;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let half_thick = if screen_pts.len() >= 3 {
                let nx = -dy / len;
                let ny = dx / len;
                let d = (screen_pts[2].x - p_left.x) * nx + (screen_pts[2].y - p_left.y) * ny;
                d.abs()
            } else {
                6.0
            };
            let nx = -dy / len * half_thick;
            let ny = dx / len * half_thick;

            match &opening.kind {
                OpeningKind::Door { .. } => {
                    draw_door_symbol(painter, p_left, p_right, is_selected);
                }
                OpeningKind::Window { .. } => {
                    draw_window_symbol(painter, p_left, p_right, nx, ny, is_selected);
                }
            }

            if is_selected && screen_pts.len() >= 3 {
                painter.add(egui::Shape::closed_line(
                    screen_pts,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 160, 255)),
                ));
            }
        }
    }

    // -----------------------------------------------------------------------
    // Drawing: Edges
    // -----------------------------------------------------------------------

    fn draw_edges(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();

        let normal_color = egui::Color32::from_rgb(160, 160, 170);
        let selected_color = egui::Color32::from_rgb(60, 160, 255);

        for edge in &self.project.edges {
            let a = match self.project.point(edge.point_a) {
                Some(p) => p,
                None => continue,
            };
            let b = match self.project.point(edge.point_b) {
                Some(p) => p,
                None => continue,
            };

            let sa = world_to_screen(&self.canvas, center, a.position);
            let sb = world_to_screen(&self.canvas, center, b.position);

            let is_selected = self.selection == Selection::Edge(edge.id);
            let (color, width) = if is_selected {
                (selected_color, 2.5)
            } else {
                (normal_color, 1.0)
            };

            painter.line_segment([sa, sb], egui::Stroke::new(width, color));
        }
    }

    // -----------------------------------------------------------------------
    // Drawing: Points
    // -----------------------------------------------------------------------

    fn draw_points(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();

        for point in &self.project.points {
            let screen = world_to_screen(&self.canvas, center, point.position);
            let is_selected = self.selection == Selection::Point(point.id);

            let radius = if is_selected { 7.0 } else { 5.0 };
            let (fill, stroke) = if is_selected {
                (
                    egui::Color32::from_rgb(0, 120, 255),
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                )
            } else {
                (
                    egui::Color32::from_rgb(200, 200, 200),
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                )
            };

            painter.circle(screen, radius, fill, stroke);
        }
    }

    // -----------------------------------------------------------------------
    // Drawing: Measurement labels
    // -----------------------------------------------------------------------

    fn draw_measurement_labels(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let label_color = egui::Color32::from_rgb(190, 190, 200);

        for edge in &self.project.edges {
            let a = match self.project.point(edge.point_a) {
                Some(p) => p,
                None => continue,
            };
            let b = match self.project.point(edge.point_b) {
                Some(p) => p,
                None => continue,
            };

            let sa = world_to_screen(&self.canvas, center, a.position);
            let sb = world_to_screen(&self.canvas, center, b.position);

            let dx = sb.x - sa.x;
            let dy = sb.y - sa.y;
            let screen_len = (dx * dx + dy * dy).sqrt();
            if screen_len < 30.0 {
                continue;
            }

            let angle = dy.atan2(dx);
            let dist_mm = edge.distance(&self.project.points);

            let label = if dist_mm >= 1000.0 {
                format!("{:.2} м", dist_mm / 1000.0)
            } else {
                format!("{:.0} мм", dist_mm)
            };

            let perp_x = -dy / screen_len * 10.0;
            let perp_y = dx / screen_len * 10.0;
            let mid = egui::pos2((sa.x + sb.x) / 2.0 + perp_x, (sa.y + sb.y) / 2.0 + perp_y);

            let color = if edge.distance_override.is_some() {
                egui::Color32::from_rgb(240, 200, 100)
            } else {
                label_color
            };

            paint_rotated_text(
                painter,
                mid,
                label,
                egui::FontId::proportional(10.0 * self.label_scale),
                color,
                angle,
            );
        }

        // Room name + area at centroid
        for (i, room) in self.project.rooms.iter().enumerate() {
            let screen_pts =
                polygon_screen_coords(&room.points, &self.project, &self.canvas, center);
            if screen_pts.is_empty() {
                continue;
            }

            let cx: f32 = screen_pts.iter().map(|p| p.x).sum::<f32>() / screen_pts.len() as f32;
            let cy: f32 = screen_pts.iter().map(|p| p.y).sum::<f32>() / screen_pts.len() as f32;

            let (r, g, b) = ROOM_COLORS[i % ROOM_COLORS.len()];
            let room_label_color = egui::Color32::from_rgb(r, g, b);

            painter.text(
                egui::pos2(cx, cy),
                egui::Align2::CENTER_CENTER,
                &room.name,
                egui::FontId::proportional(13.0 * self.label_scale),
                room_label_color,
            );

            let area_m2 = room.floor_area(&self.project) / 1_000_000.0;
            painter.text(
                egui::pos2(cx, cy + 16.0 * self.label_scale),
                egui::Align2::CENTER_CENTER,
                format!("{:.1} м²", area_m2),
                egui::FontId::proportional(11.0 * self.label_scale),
                room_label_color,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Drawing: User labels
    // -----------------------------------------------------------------------

    fn draw_labels(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();

        let normal_color = egui::Color32::from_rgb(220, 220, 225);
        let selected_color = egui::Color32::from_rgb(255, 255, 255);

        for label in &self.project.labels {
            if label.text.is_empty() {
                continue;
            }
            let screen_pos = world_to_screen(&self.canvas, center, label.position);
            let is_selected = self.selection == Selection::Label(label.id);
            let color = if is_selected {
                selected_color
            } else {
                normal_color
            };
            let font_size = label.font_size as f32 * self.label_scale;

            paint_rotated_text(
                painter,
                screen_pos,
                label.text.clone(),
                egui::FontId::proportional(font_size),
                color,
                label.rotation as f32,
            );

            if is_selected {
                let galley = painter.layout_no_wrap(
                    label.text.clone(),
                    egui::FontId::proportional(font_size),
                    color,
                );
                let w = galley.size().x;
                let h = galley.size().y;
                let pad = 3.0;
                let sel_rect = egui::Rect::from_center_size(
                    screen_pos,
                    egui::vec2(w + pad * 2.0, h + pad * 2.0),
                );
                painter.rect_stroke(
                    sel_rect,
                    2.0,
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 160, 255)),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Drawing: Tool preview
    // -----------------------------------------------------------------------

    fn draw_tool_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let cursor_world = match self.canvas.cursor_world_pos {
            Some(p) => DVec2::new(p.x as f64, p.y as f64),
            None => return,
        };
        let cursor_screen = world_to_screen(&self.canvas, center, cursor_world);

        let color = match self.active_tool {
            Tool::Room => egui::Color32::from_rgba_premultiplied(70, 180, 130, 160),
            Tool::Wall => egui::Color32::from_rgba_premultiplied(180, 180, 180, 160),
            Tool::Door => egui::Color32::from_rgba_premultiplied(180, 120, 60, 160),
            Tool::Window => egui::Color32::from_rgba_premultiplied(80, 160, 220, 160),
            _ => return,
        };
        self.draw_polygon_preview(
            painter,
            center,
            cursor_screen,
            &self.tool_state.points,
            color,
        );
    }

    fn draw_polygon_preview(
        &self,
        painter: &egui::Painter,
        center: egui::Pos2,
        cursor_screen: egui::Pos2,
        point_ids: &[uuid::Uuid],
        color: egui::Color32,
    ) {
        if point_ids.is_empty() {
            return;
        }

        let screen_pts =
            polygon_screen_coords(point_ids, &self.project, &self.canvas, center);
        if screen_pts.is_empty() {
            return;
        }

        for i in 0..screen_pts.len().saturating_sub(1) {
            painter.line_segment(
                [screen_pts[i], screen_pts[i + 1]],
                egui::Stroke::new(2.0, color),
            );
        }

        if let Some(&last) = screen_pts.last() {
            painter.line_segment([last, cursor_screen], egui::Stroke::new(1.5, color));
        }

        for (i, sp) in screen_pts.iter().enumerate() {
            let r = if i == 0 { 6.0 } else { 4.0 };
            painter.circle_filled(*sp, r, color);
        }

        // Close indicator
        if screen_pts.len() >= 3 {
            let dist_to_first = ((cursor_screen.x - screen_pts[0].x).powi(2)
                + (cursor_screen.y - screen_pts[0].y).powi(2))
            .sqrt();
            if dist_to_first < 15.0 {
                painter.circle_stroke(screen_pts[0], 10.0, egui::Stroke::new(2.0, color));
            }
        }
    }
}
