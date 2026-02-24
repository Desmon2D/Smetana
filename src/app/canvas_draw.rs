use eframe::egui;
use glam::DVec2;

use super::App;
use crate::editor::{Canvas, Selection, Tool};
use crate::model::{OpeningKind, Project};

// ---------------------------------------------------------------------------
// Coordinate helpers
// ---------------------------------------------------------------------------

/// Convert a world-space DVec2 (mm) to screen-space Pos2 via the canvas.
fn world_to_screen(canvas: &Canvas, center: egui::Pos2, p: DVec2) -> egui::Pos2 {
    canvas.world_to_screen(egui::pos2(p.x as f32, p.y as f32), center)
}

/// Collect screen-space positions for a polygon defined by point IDs.
fn polygon_screen_coords(
    point_ids: &[uuid::Uuid],
    project: &Project,
    canvas: &Canvas,
    center: egui::Pos2,
) -> Vec<egui::Pos2> {
    point_ids
        .iter()
        .filter_map(|id| project.point(*id))
        .map(|p| world_to_screen(canvas, center, p.position))
        .collect()
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

/// Draw text centered at `center_pos`, rotated to follow a given angle.
/// The angle is automatically flipped so text is never upside-down.
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
// Triangulation helper
// ---------------------------------------------------------------------------

/// Triangulate a simple polygon using earcutr (earcut algorithm).
fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]> {
    if vertices.len() < 3 {
        return Vec::new();
    }
    let coords: Vec<f32> = vertices.iter().flat_map(|p| [p.x, p.y]).collect();
    let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
    indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()
}

// ---------------------------------------------------------------------------
// Door / Window symbol rendering
// ---------------------------------------------------------------------------

/// Draw a door symbol: a straight line plus a 90-degree swing arc.
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

/// Draw a window symbol: two parallel lines offset from centerline plus
/// short perpendicular end-caps.
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
// App drawing methods
// ---------------------------------------------------------------------------

impl App {
    // -----------------------------------------------------------------------
    // 1. Room fills — triangulated polygons with cutout holes
    // -----------------------------------------------------------------------

    pub(super) fn draw_room_fills(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.editor.visibility.show_room_fills() {
            return;
        }

        let center = rect.center();

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

        let selected_room_id = match self.editor.selection {
            Selection::Room(id) => Some(id),
            _ => None,
        };

        for (i, room) in self.project.rooms.iter().enumerate() {
            let screen_pts =
                polygon_screen_coords(&room.points, &self.project, &self.editor.canvas, center);
            if screen_pts.len() < 3 {
                continue;
            }

            let (r, g, b) = ROOM_COLORS[i % ROOM_COLORS.len()];
            let alpha = if selected_room_id == Some(room.id) {
                60
            } else {
                40
            };
            let fill = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);

            if room.cutouts.is_empty() {
                // Simple triangulation
                let triangles = triangulate(&screen_pts);
                for tri in &triangles {
                    let tri_pts = vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]];
                    painter.add(egui::Shape::convex_polygon(
                        tri_pts,
                        fill,
                        egui::Stroke::NONE,
                    ));
                }
            } else {
                // Triangulation with holes
                let mut coords: Vec<f32> = screen_pts.iter().flat_map(|p| [p.x, p.y]).collect();
                let mut hole_indices = Vec::new();

                for cutout in &room.cutouts {
                    let cutout_pts =
                        polygon_screen_coords(cutout, &self.project, &self.editor.canvas, center);
                    if cutout_pts.len() < 3 {
                        continue;
                    }
                    hole_indices.push(coords.len() / 2);
                    // Reverse winding for hole
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
            let stroke_w = if selected_room_id == Some(room.id) {
                2.0
            } else {
                1.0
            };
            painter.add(egui::Shape::closed_line(
                screen_pts,
                egui::Stroke::new(stroke_w, outline_color),
            ));
        }
    }

    // -----------------------------------------------------------------------
    // 2. Wall fills — simple filled polygons
    // -----------------------------------------------------------------------

    pub(super) fn draw_wall_fills(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.editor.visibility.show_wall_fills() {
            return;
        }

        let center = rect.center();
        let wall_outline = egui::Color32::from_rgb(40, 40, 42);

        let selected_wall_id = match self.editor.selection {
            Selection::Wall(id) => Some(id),
            _ => None,
        };

        for wall in &self.project.walls {
            let screen_pts =
                polygon_screen_coords(&wall.points, &self.project, &self.editor.canvas, center);
            if screen_pts.len() < 3 {
                continue;
            }

            let is_selected = selected_wall_id == Some(wall.id);
            let [r, g, b, a] = wall.color;
            let fill = if is_selected {
                // Brighten when selected
                egui::Color32::from_rgba_unmultiplied(
                    r.saturating_add(40),
                    g.saturating_add(40),
                    b.saturating_add(40),
                    a,
                )
            } else {
                egui::Color32::from_rgba_unmultiplied(r, g, b, a)
            };

            let triangles = triangulate(&screen_pts);
            for tri in &triangles {
                let tri_pts = vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]];
                painter.add(egui::Shape::convex_polygon(
                    tri_pts,
                    fill,
                    egui::Stroke::NONE,
                ));
            }

            // Outline
            let outline_stroke = if is_selected {
                egui::Stroke::new(2.5, egui::Color32::from_rgb(60, 160, 255))
            } else {
                egui::Stroke::new(1.0, wall_outline)
            };
            painter.add(egui::Shape::closed_line(screen_pts, outline_stroke));
        }
    }

    // -----------------------------------------------------------------------
    // 3. Opening fills — filled polygons with door/window symbols
    // -----------------------------------------------------------------------

    pub(super) fn draw_opening_fills(&self, painter: &egui::Painter, rect: egui::Rect) {
        if !self.editor.visibility.show_opening_fills() {
            return;
        }

        let center = rect.center();

        let selected_opening_id = match self.editor.selection {
            Selection::Opening(id) => Some(id),
            _ => None,
        };

        for opening in &self.project.openings {
            let screen_pts =
                polygon_screen_coords(&opening.points, &self.project, &self.editor.canvas, center);
            if screen_pts.len() < 2 {
                continue;
            }

            let is_selected = selected_opening_id == Some(opening.id);

            // Draw the opening gap (background cutout)
            if screen_pts.len() >= 3 {
                let canvas_bg = egui::Color32::from_rgb(45, 45, 48);
                let triangles = triangulate(&screen_pts);
                for tri in &triangles {
                    let tri_pts = vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]];
                    painter.add(egui::Shape::convex_polygon(
                        tri_pts,
                        canvas_bg,
                        egui::Stroke::NONE,
                    ));
                }
            }

            // Draw the symbol along the first edge (points[0] → points[1])
            let p_left = screen_pts[0];
            let p_right = screen_pts[1];

            // Compute normal for window symbol
            let dx = p_right.x - p_left.x;
            let dy = p_right.y - p_left.y;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let half_thick = if screen_pts.len() >= 3 {
                // Estimate thickness from the polygon perpendicular extent
                let nx = -dy / len;
                let ny = dx / len;
                // Project third point onto normal to estimate half-thickness
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

            // Selection outline
            if is_selected && screen_pts.len() >= 3 {
                painter.add(egui::Shape::closed_line(
                    screen_pts,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 160, 255)),
                ));
            }
        }
    }

    // -----------------------------------------------------------------------
    // 4. Edges — lines between point pairs
    // -----------------------------------------------------------------------

    pub(super) fn draw_edges(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();

        let selected_edge_id = match self.editor.selection {
            Selection::Edge(id) => Some(id),
            _ => None,
        };

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

            let sa = world_to_screen(&self.editor.canvas, center, a.position);
            let sb = world_to_screen(&self.editor.canvas, center, b.position);

            let is_selected = selected_edge_id == Some(edge.id);
            let (color, width) = if is_selected {
                (selected_color, 2.5)
            } else {
                (normal_color, 1.0)
            };

            painter.line_segment([sa, sb], egui::Stroke::new(width, color));
        }
    }

    // -----------------------------------------------------------------------
    // 5. Points — circles, always on top
    // -----------------------------------------------------------------------

    pub(super) fn draw_points(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();

        let selected_point_id = match self.editor.selection {
            Selection::Point(id) => Some(id),
            _ => None,
        };

        for point in &self.project.points {
            let screen = world_to_screen(&self.editor.canvas, center, point.position);
            let is_selected = selected_point_id == Some(point.id);

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
    // 6. Measurement labels — edge distances, room name + area
    // -----------------------------------------------------------------------

    pub(super) fn draw_measurement_labels(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let label_color = egui::Color32::from_rgb(190, 190, 200);

        // Edge distance labels at midpoint
        for edge in &self.project.edges {
            let a = match self.project.point(edge.point_a) {
                Some(p) => p,
                None => continue,
            };
            let b = match self.project.point(edge.point_b) {
                Some(p) => p,
                None => continue,
            };

            let sa = world_to_screen(&self.editor.canvas, center, a.position);
            let sb = world_to_screen(&self.editor.canvas, center, b.position);

            let dx = sb.x - sa.x;
            let dy = sb.y - sa.y;
            let screen_len = (dx * dx + dy * dy).sqrt();
            if screen_len < 30.0 {
                continue; // Too short to label
            }

            let angle = dy.atan2(dx);
            let dist_mm = edge.distance(&self.project.points);

            let label = if dist_mm >= 1000.0 {
                format!("{:.2} м", dist_mm / 1000.0)
            } else {
                format!("{:.0} мм", dist_mm)
            };

            // Offset perpendicular to the edge so the label doesn't overlap the line
            let perp_x = -dy / screen_len * 10.0;
            let perp_y = dx / screen_len * 10.0;
            let mid = egui::pos2((sa.x + sb.x) / 2.0 + perp_x, (sa.y + sb.y) / 2.0 + perp_y);

            // Show override indicator
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

        for (i, room) in self.project.rooms.iter().enumerate() {
            let screen_pts =
                polygon_screen_coords(&room.points, &self.project, &self.editor.canvas, center);
            if screen_pts.is_empty() {
                continue;
            }

            let cx: f32 = screen_pts.iter().map(|p| p.x).sum::<f32>() / screen_pts.len() as f32;
            let cy: f32 = screen_pts.iter().map(|p| p.y).sum::<f32>() / screen_pts.len() as f32;

            let (r, g, b) = ROOM_COLORS[i % ROOM_COLORS.len()];
            let room_label_color = egui::Color32::from_rgb(r, g, b);

            // Room name
            painter.text(
                egui::pos2(cx, cy),
                egui::Align2::CENTER_CENTER,
                &room.name,
                egui::FontId::proportional(13.0 * self.label_scale),
                room_label_color,
            );

            // Floor area
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
    // 7. User labels
    // -----------------------------------------------------------------------

    pub(super) fn draw_labels(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let selected_label_id = match self.editor.selection {
            Selection::Label(id) => Some(id),
            _ => None,
        };

        let normal_color = egui::Color32::from_rgb(220, 220, 225);
        let selected_color = egui::Color32::from_rgb(255, 255, 255);

        for label in &self.project.labels {
            if label.text.is_empty() {
                continue;
            }
            let screen_pos = world_to_screen(&self.editor.canvas, center, label.position);
            let is_selected = selected_label_id == Some(label.id);
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
    // 8. Tool preview — ghost polygon for in-progress tools
    // -----------------------------------------------------------------------

    pub(super) fn draw_tool_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let cursor_world = match self.editor.canvas.cursor_world_pos {
            Some(p) => DVec2::new(p.x as f64, p.y as f64),
            None => return,
        };
        let cursor_screen = world_to_screen(&self.editor.canvas, center, cursor_world);

        match self.editor.active_tool {
            Tool::Room => {
                self.draw_polygon_preview(
                    painter,
                    center,
                    cursor_screen,
                    &self.editor.room_tool.points,
                    egui::Color32::from_rgba_premultiplied(70, 180, 130, 160),
                    true, // close to first point
                );
            }
            Tool::Wall => {
                self.draw_polygon_preview(
                    painter,
                    center,
                    cursor_screen,
                    &self.editor.polygon_tool.points,
                    egui::Color32::from_rgba_premultiplied(180, 180, 180, 160),
                    true,
                );
            }
            Tool::Door => {
                self.draw_polygon_preview(
                    painter,
                    center,
                    cursor_screen,
                    &self.editor.polygon_tool.points,
                    egui::Color32::from_rgba_premultiplied(180, 120, 60, 160),
                    true,
                );
            }
            Tool::Window => {
                self.draw_polygon_preview(
                    painter,
                    center,
                    cursor_screen,
                    &self.editor.polygon_tool.points,
                    egui::Color32::from_rgba_premultiplied(80, 160, 220, 160),
                    true,
                );
            }
            _ => {}
        }
    }

    /// Draw a ghost polygon preview for polygon-based tools.
    fn draw_polygon_preview(
        &self,
        painter: &egui::Painter,
        center: egui::Pos2,
        cursor_screen: egui::Pos2,
        point_ids: &[uuid::Uuid],
        color: egui::Color32,
        show_close_indicator: bool,
    ) {
        if point_ids.is_empty() {
            return;
        }

        let screen_pts =
            polygon_screen_coords(point_ids, &self.project, &self.editor.canvas, center);
        if screen_pts.is_empty() {
            return;
        }

        // Draw collected edges
        for i in 0..screen_pts.len().saturating_sub(1) {
            painter.line_segment(
                [screen_pts[i], screen_pts[i + 1]],
                egui::Stroke::new(2.0, color),
            );
        }

        // Line from last point to cursor
        if let Some(&last) = screen_pts.last() {
            painter.line_segment([last, cursor_screen], egui::Stroke::new(1.5, color));
        }

        // Mark collected points
        for (i, sp) in screen_pts.iter().enumerate() {
            let r = if i == 0 { 6.0 } else { 4.0 };
            painter.circle_filled(*sp, r, color);
        }

        // Close indicator: if cursor is near the first point and enough points collected
        if show_close_indicator && screen_pts.len() >= 3 {
            let dist_to_first = ((cursor_screen.x - screen_pts[0].x).powi(2)
                + (cursor_screen.y - screen_pts[0].y).powi(2))
            .sqrt();
            if dist_to_first < 15.0 {
                painter.circle_stroke(screen_pts[0], 10.0, egui::Stroke::new(2.0, color));
            }
        }
    }
}
