use eframe::egui;

use super::canvas::PointPreviewKind;
use super::viewport::{Canvas, VisibilityMode};
use super::{Selection, Tool};
use crate::model::{OpeningKind, Project};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub(super) const ROOM_COLORS: &[(u8, u8, u8)] = &[
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
// DrawCtx
// ---------------------------------------------------------------------------

pub(super) struct DrawCtx<'a> {
    pub painter: &'a egui::Painter,
    pub center: egui::Pos2,
    pub canvas: &'a Canvas,
    pub project: &'a Project,
    pub selection: Selection,
    pub hover: Selection,
    pub visibility: VisibilityMode,
    pub label_scale: f32,
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

pub(super) fn polygon_screen_coords(
    point_ids: &[uuid::Uuid],
    project: &Project,
    canvas: &Canvas,
    center: egui::Pos2,
) -> Vec<egui::Pos2> {
    project
        .resolve_positions(point_ids)
        .iter()
        .map(|&pos| canvas.dvec2_to_screen(pos, center))
        .collect()
}

/// Render text centered at `center_pos` with rotation. Returns the galley size.
pub(super) fn paint_rotated_text(
    painter: &egui::Painter,
    center_pos: egui::Pos2,
    text: String,
    font_id: egui::FontId,
    color: egui::Color32,
    angle_rad: f32,
) -> egui::Vec2 {
    let angle = if angle_rad > std::f32::consts::FRAC_PI_2 {
        angle_rad - std::f32::consts::PI
    } else if angle_rad < -std::f32::consts::FRAC_PI_2 {
        angle_rad + std::f32::consts::PI
    } else {
        angle_rad
    };

    let galley = painter.layout_no_wrap(text, font_id, color);
    let size = galley.size();
    let w = size.x;
    let h = size.y;

    let (sin_a, cos_a) = angle.sin_cos();
    let offset_x = cos_a * (w / 2.0) - sin_a * (h / 2.0);
    let offset_y = sin_a * (w / 2.0) + cos_a * (h / 2.0);
    let pos = egui::pos2(center_pos.x - offset_x, center_pos.y - offset_y);

    let text_shape = egui::epaint::TextShape::new(pos, galley, color).with_angle(angle);
    painter.add(text_shape);
    size
}

fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]> {
    if vertices.len() < 3 {
        return Vec::new();
    }
    let coords: Vec<f32> = vertices.iter().flat_map(|p| [p.x, p.y]).collect();
    let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
    indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()
}

fn fill_polygon(painter: &egui::Painter, screen_pts: &[egui::Pos2], fill: egui::Color32) {
    for tri in &triangulate(screen_pts) {
        painter.add(egui::Shape::convex_polygon(
            vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]],
            fill,
            egui::Stroke::NONE,
        ));
    }
}

fn fill_polygon_with_holes(
    painter: &egui::Painter,
    outer: &[egui::Pos2],
    holes: &[Vec<egui::Pos2>],
    fill: egui::Color32,
) {
    if holes.is_empty() {
        fill_polygon(painter, outer, fill);
        return;
    }
    let mut coords: Vec<f32> = outer.iter().flat_map(|p| [p.x, p.y]).collect();
    let mut hole_indices = Vec::new();
    for hole in holes {
        if hole.len() < 3 {
            continue;
        }
        hole_indices.push(coords.len() / 2);
        for p in hole.iter().rev() {
            coords.extend([p.x, p.y]);
        }
    }
    let all_pts: Vec<egui::Pos2> = coords.chunks(2).map(|c| egui::pos2(c[0], c[1])).collect();
    let indices = earcutr::earcut(&coords, &hole_indices, 2).unwrap_or_default();
    for tri in indices.chunks(3) {
        if tri.len() == 3 {
            painter.add(egui::Shape::convex_polygon(
                vec![all_pts[tri[0]], all_pts[tri[1]], all_pts[tri[2]]],
                fill,
                egui::Stroke::NONE,
            ));
        }
    }
}

fn draw_door_symbol(
    painter: &egui::Painter,
    p_left: egui::Pos2,
    p_right: egui::Pos2,
    arc_sign: f32,
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
        let px = -uy * arc_sign;
        let py = ux * arc_sign;

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
    inx: f32,
    iny: f32,
    is_selected: bool,
) {
    let color = if is_selected {
        egui::Color32::from_rgb(120, 210, 255)
    } else {
        egui::Color32::from_rgb(80, 160, 220)
    };
    let stroke_w = if is_selected { 2.0 } else { 1.5 };

    // inx, iny point toward polygon interior with full thickness magnitude.
    // Draw two parallel lines centered within the polygon (at 35% and 65%).
    for t in [0.35_f32, 0.65_f32] {
        let ox = inx * t;
        let oy = iny * t;
        painter.line_segment(
            [
                egui::pos2(p_left.x + ox, p_left.y + oy),
                egui::pos2(p_right.x + ox, p_right.y + oy),
            ],
            egui::Stroke::new(stroke_w, color),
        );
    }

    // End caps connecting the two parallel lines
    for p in [p_left, p_right] {
        painter.line_segment(
            [
                egui::pos2(p.x + inx * 0.35, p.y + iny * 0.35),
                egui::pos2(p.x + inx * 0.65, p.y + iny * 0.65),
            ],
            egui::Stroke::new(stroke_w, color),
        );
    }
}

// ---------------------------------------------------------------------------
// Edge style helpers
// ---------------------------------------------------------------------------

fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: egui::Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let total = (dx * dx + dy * dy).sqrt();
    if total < 0.01 {
        return;
    }
    let ux = dx / total;
    let uy = dy / total;

    let mut dist = 0.0;
    let mut drawing = true;
    while dist < total {
        let seg = if drawing { dash_len } else { gap_len };
        let end_dist = (dist + seg).min(total);
        if drawing {
            let p0 = egui::pos2(from.x + ux * dist, from.y + uy * dist);
            let p1 = egui::pos2(from.x + ux * end_dist, from.y + uy * end_dist);
            painter.line_segment([p0, p1], stroke);
        }
        dist = end_dist;
        drawing = !drawing;
    }
}

fn draw_arrowhead(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    line_width: f32,
    color: egui::Color32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;

    let arrow_len = (8.0 + line_width * 2.0).min(len * 0.4);
    let arrow_half_w = arrow_len * 0.4;

    let tip = to;
    let base_x = tip.x - ux * arrow_len;
    let base_y = tip.y - uy * arrow_len;
    let left = egui::pos2(base_x + uy * arrow_half_w, base_y - ux * arrow_half_w);
    let right = egui::pos2(base_x - uy * arrow_half_w, base_y + ux * arrow_half_w);

    painter.add(egui::Shape::convex_polygon(
        vec![tip, left, right],
        color,
        egui::Stroke::NONE,
    ));
}

// ---------------------------------------------------------------------------
// DrawCtx methods
// ---------------------------------------------------------------------------

impl DrawCtx<'_> {
    pub fn draw_room_fills(&self) {
        if !self.visibility.show_room_fills() {
            return;
        }

        for room in &self.project.rooms {
            let screen_pts =
                polygon_screen_coords(&room.points, self.project, self.canvas, self.center);
            if screen_pts.len() < 3 {
                continue;
            }

            let [r, g, b, base_alpha] = room.color;
            let is_selected = self.selection == Selection::Room(room.id);
            let is_hovered = self.hover == Selection::Room(room.id);
            let alpha = if is_selected {
                base_alpha.saturating_add(20)
            } else if is_hovered {
                base_alpha.saturating_add(15)
            } else {
                base_alpha
            };
            let fill = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);

            let hole_pts: Vec<Vec<egui::Pos2>> = room
                .cutouts
                .iter()
                .map(|c| polygon_screen_coords(c, self.project, self.canvas, self.center))
                .collect();
            fill_polygon_with_holes(self.painter, &screen_pts, &hole_pts, fill);

            let (outline_color, stroke_w) = if is_selected {
                (egui::Color32::from_rgba_unmultiplied(r, g, b, 80), 2.0)
            } else if is_hovered {
                (egui::Color32::from_rgba_unmultiplied(r, g, b, 120), 1.5)
            } else {
                (egui::Color32::from_rgba_unmultiplied(r, g, b, 80), 1.0)
            };
            self.painter.add(egui::Shape::closed_line(
                screen_pts,
                egui::Stroke::new(stroke_w, outline_color),
            ));
        }
    }

    pub fn draw_wall_fills(&self) {
        if !self.visibility.show_wall_fills() {
            return;
        }

        let wall_outline = egui::Color32::from_rgb(40, 40, 42);

        for wall in &self.project.walls {
            let screen_pts =
                polygon_screen_coords(&wall.points, self.project, self.canvas, self.center);
            if screen_pts.len() < 3 {
                continue;
            }

            let is_selected = self.selection == Selection::Wall(wall.id);
            let is_hovered = self.hover == Selection::Wall(wall.id);
            let [r, g, b, a] = wall.color;
            let fill = if is_selected {
                egui::Color32::from_rgba_unmultiplied(
                    r.saturating_add(40),
                    g.saturating_add(40),
                    b.saturating_add(40),
                    a,
                )
            } else if is_hovered {
                egui::Color32::from_rgba_unmultiplied(
                    r.saturating_add(20),
                    g.saturating_add(20),
                    b.saturating_add(20),
                    a,
                )
            } else {
                egui::Color32::from_rgba_unmultiplied(r, g, b, a)
            };

            fill_polygon(self.painter, &screen_pts, fill);

            let outline_stroke = if is_selected {
                egui::Stroke::new(2.5, egui::Color32::from_rgb(60, 160, 255))
            } else if is_hovered {
                egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 180, 255))
            } else {
                egui::Stroke::new(1.0, wall_outline)
            };
            self.painter
                .add(egui::Shape::closed_line(screen_pts, outline_stroke));
        }
    }

    pub fn draw_opening_fills(&self) {
        if !self.visibility.show_opening_fills() {
            return;
        }

        for opening in &self.project.openings {
            let screen_pts =
                polygon_screen_coords(&opening.points, self.project, self.canvas, self.center);
            if screen_pts.len() < 2 {
                continue;
            }

            let is_selected = self.selection == Selection::Opening(opening.id);
            let is_hovered = self.hover == Selection::Opening(opening.id);

            if screen_pts.len() >= 3 {
                let [r, g, b, a] = opening.color;
                let fill = if is_selected {
                    egui::Color32::from_rgba_unmultiplied(
                        r.saturating_add(40),
                        g.saturating_add(40),
                        b.saturating_add(40),
                        a,
                    )
                } else if is_hovered {
                    egui::Color32::from_rgba_unmultiplied(
                        r.saturating_add(20),
                        g.saturating_add(20),
                        b.saturating_add(20),
                        a,
                    )
                } else {
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
                };
                fill_polygon(self.painter, &screen_pts, fill);
            }

            match &opening.kind {
                OpeningKind::Door { swing_edge, swing_outward, swing_mirrored, .. } => {
                    let n = screen_pts.len();
                    let idx = *swing_edge % n;
                    let (p_a, p_b) = if *swing_mirrored {
                        (screen_pts[(idx + 1) % n], screen_pts[idx])
                    } else {
                        (screen_pts[idx], screen_pts[(idx + 1) % n])
                    };

                    // Compute interior direction for this edge
                    let dx = p_b.x - p_a.x;
                    let dy = p_b.y - p_a.y;
                    let len = (dx * dx + dy * dy).sqrt().max(0.001);
                    let nx = -dy / len;
                    let ny = dx / len;

                    // Centroid determines which side is interior
                    let cx = screen_pts.iter().map(|p| p.x).sum::<f32>() / n as f32;
                    let cy = screen_pts.iter().map(|p| p.y).sum::<f32>() / n as f32;
                    let mid_x = (p_a.x + p_b.x) / 2.0;
                    let mid_y = (p_a.y + p_b.y) / 2.0;
                    let to_center = (cx - mid_x) * nx + (cy - mid_y) * ny;
                    let interior_sign = if to_center >= 0.0 { 1.0_f32 } else { -1.0 };
                    let arc_sign = if *swing_outward { -interior_sign } else { interior_sign };

                    draw_door_symbol(self.painter, p_a, p_b, arc_sign, is_selected);
                }
                OpeningKind::Window { .. } => {
                    let p_left = screen_pts[0];
                    let p_right = screen_pts[1];
                    let dx = p_right.x - p_left.x;
                    let dy = p_right.y - p_left.y;
                    let len = (dx * dx + dy * dy).sqrt().max(0.001);
                    let unit_nx = -dy / len;
                    let unit_ny = dx / len;
                    let (thickness, sign) = if screen_pts.len() >= 3 {
                        let d = (screen_pts[2].x - p_left.x) * unit_nx
                            + (screen_pts[2].y - p_left.y) * unit_ny;
                        (d.abs(), if d >= 0.0 { 1.0_f32 } else { -1.0 })
                    } else {
                        (6.0, 1.0)
                    };
                    let inx = unit_nx * sign * thickness;
                    let iny = unit_ny * sign * thickness;
                    draw_window_symbol(self.painter, p_left, p_right, inx, iny, is_selected);
                }
            }

            if (is_selected || is_hovered) && screen_pts.len() >= 3 {
                let stroke = if is_selected {
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 160, 255))
                } else {
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 180, 255))
                };
                self.painter
                    .add(egui::Shape::closed_line(screen_pts, stroke));
            }
        }
    }

    pub fn draw_edges(&self) {
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

            let sa = self.canvas.dvec2_to_screen(a.position, self.center);
            let sb = self.canvas.dvec2_to_screen(b.position, self.center);

            let is_selected = self.selection == Selection::Edge(edge.id);
            let is_hovered = self.hover == Selection::Edge(edge.id);
            let (color, width) = if is_selected {
                (selected_color, 2.5)
            } else if is_hovered {
                (egui::Color32::from_rgb(100, 180, 255), 2.0)
            } else {
                (normal_color, 1.0)
            };

            let stroke = egui::Stroke::new(width, color);

            match edge.line_pattern {
                crate::model::LinePattern::Dashed => draw_dashed_line(self.painter, sa, sb, stroke, 10.0, 5.0),
                crate::model::LinePattern::Dotted => draw_dashed_line(self.painter, sa, sb, stroke, 2.0, 4.0),
                crate::model::LinePattern::Solid => { self.painter.line_segment([sa, sb], stroke); },
            }

            if edge.arrow_mode.forward() {
                draw_arrowhead(self.painter, sa, sb, width, color);
            }
            if edge.arrow_mode.backward() {
                draw_arrowhead(self.painter, sb, sa, width, color);
            }
        }
    }

    pub fn draw_points(&self) {
        for point in &self.project.points {
            let screen = self.canvas.dvec2_to_screen(point.position, self.center);
            let is_selected = self.selection == Selection::Point(point.id);
            let is_hovered = self.hover == Selection::Point(point.id);

            let radius = if is_selected { 7.0 } else if is_hovered { 6.0 } else { 5.0 };
            let (fill, stroke) = if is_selected {
                (
                    egui::Color32::from_rgb(0, 120, 255),
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                )
            } else if is_hovered {
                (
                    egui::Color32::from_rgb(230, 230, 240),
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 180, 255)),
                )
            } else {
                (
                    egui::Color32::from_rgb(200, 200, 200),
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                )
            };

            self.painter.circle(screen, radius, fill, stroke);
        }
    }

    pub fn draw_measurement_labels(&self) {
        let label_color = egui::Color32::from_rgb(190, 190, 200);

        for edge in &self.project.edges {
            if edge.label_hidden {
                continue;
            }

            let a = match self.project.point(edge.point_a) {
                Some(p) => p,
                None => continue,
            };
            let b = match self.project.point(edge.point_b) {
                Some(p) => p,
                None => continue,
            };

            let sa = self.canvas.dvec2_to_screen(a.position, self.center);
            let sb = self.canvas.dvec2_to_screen(b.position, self.center);

            let dx = sb.x - sa.x;
            let dy = sb.y - sa.y;
            let screen_len = (dx * dx + dy * dy).sqrt();
            if screen_len < 30.0 {
                continue;
            }

            let mut angle = dy.atan2(dx);
            // Normalize to prevent upside-down text
            if angle > std::f32::consts::FRAC_PI_2 {
                angle -= std::f32::consts::PI;
            } else if angle < -std::f32::consts::FRAC_PI_2 {
                angle += std::f32::consts::PI;
            }
            // Flip 180° if requested (after normalization, so it actually flips)
            if edge.label_flip_text {
                angle += std::f32::consts::PI;
            }

            let dist_mm = edge.distance(&self.project.points);
            let height_a = a.height;
            let height_b = b.height;
            let avg_height = (height_a + height_b) / 2.0;
            let wall_area_gross = dist_mm * avg_height;
            let openings_area =
                self.project.openings_area_on_edge(edge.point_a, edge.point_b);
            let wall_area_net_m2 = (wall_area_gross - openings_area).max(0.0) / 1_000_000.0;

            let dist_str = if dist_mm >= 1000.0 {
                format!("{:.4} м", dist_mm / 1000.0)
            } else {
                format!("{:.1} мм", dist_mm)
            };
            let label = format!("{} - {:.4} м²", dist_str, wall_area_net_m2);

            let side = if edge.label_flip_side { -1.0_f32 } else { 1.0 };
            let perp_x = -dy / screen_len * 10.0 * side;
            let perp_y = dx / screen_len * 10.0 * side;
            let mid =
                egui::pos2((sa.x + sb.x) / 2.0 + perp_x, (sa.y + sb.y) / 2.0 + perp_y);

            let color = if edge.distance_override.is_some() {
                egui::Color32::from_rgb(240, 200, 100)
            } else {
                label_color
            };

            // Render directly with final angle (bypass paint_rotated_text normalization)
            let font_id = egui::FontId::proportional(10.0 * self.label_scale);
            let galley = self.painter.layout_no_wrap(label, font_id, color);
            let size = galley.size();
            let (sin_a, cos_a) = angle.sin_cos();
            let offset_x = cos_a * (size.x / 2.0) - sin_a * (size.y / 2.0);
            let offset_y = sin_a * (size.x / 2.0) + cos_a * (size.y / 2.0);
            let pos = egui::pos2(mid.x - offset_x, mid.y - offset_y);
            let text_shape =
                egui::epaint::TextShape::new(pos, galley, color).with_angle(angle);
            self.painter.add(text_shape);
        }

        // Room name + area at centroid
        for room in &self.project.rooms {
            let screen_pts =
                polygon_screen_coords(&room.points, self.project, self.canvas, self.center);
            if screen_pts.is_empty() {
                continue;
            }

            let cx: f32 =
                screen_pts.iter().map(|p| p.x).sum::<f32>() / screen_pts.len() as f32;
            let cy: f32 =
                screen_pts.iter().map(|p| p.y).sum::<f32>() / screen_pts.len() as f32;

            let [r, g, b, _] = room.color;
            let room_label_color = egui::Color32::from_rgb(r, g, b);

            self.painter.text(
                egui::pos2(cx, cy),
                egui::Align2::CENTER_CENTER,
                &room.name,
                egui::FontId::proportional(13.0 * self.label_scale),
                room_label_color,
            );

            let area_m2 = room.floor_area(self.project) / 1_000_000.0;
            self.painter.text(
                egui::pos2(cx, cy + 16.0 * self.label_scale),
                egui::Align2::CENTER_CENTER,
                format!("{:.4} м²", area_m2),
                egui::FontId::proportional(11.0 * self.label_scale),
                room_label_color,
            );
        }
    }

    pub fn draw_labels(&self) {
        let normal_color = egui::Color32::from_rgb(220, 220, 225);
        let selected_color = egui::Color32::from_rgb(255, 255, 255);

        for label in &self.project.labels {
            if label.text.is_empty() {
                continue;
            }
            let screen_pos = self.canvas.dvec2_to_screen(label.position, self.center);
            let is_selected = self.selection == Selection::Label(label.id);
            let is_hovered = self.hover == Selection::Label(label.id);
            let color = if is_selected {
                selected_color
            } else if is_hovered {
                egui::Color32::from_rgb(240, 240, 245)
            } else {
                normal_color
            };
            let font_size = label.font_size as f32 * self.label_scale;

            let size = paint_rotated_text(
                self.painter,
                screen_pos,
                label.text.clone(),
                egui::FontId::proportional(font_size),
                color,
                label.rotation as f32,
            );

            if is_selected || is_hovered {
                let pad = 3.0;
                let sel_rect = egui::Rect::from_center_size(
                    screen_pos,
                    egui::vec2(size.x + pad * 2.0, size.y + pad * 2.0),
                );
                let stroke = if is_selected {
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 160, 255))
                } else {
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 180, 255))
                };
                self.painter.rect_stroke(
                    sel_rect,
                    2.0,
                    stroke,
                    egui::StrokeKind::Outside,
                );
            }
        }
    }

    pub fn draw_tool_preview(
        &self,
        active_tool: Tool,
        tool_points: &[uuid::Uuid],
        point_preview: Option<(glam::DVec2, PointPreviewKind)>,
    ) {
        // Edge tool preview
        if active_tool == Tool::Edge {
            if tool_points.len() == 1
                && let Some(cursor_world) = self.canvas.cursor_world_pos
                && let Some(first) = self.project.point(tool_points[0])
            {
                let first_screen = self.canvas.dvec2_to_screen(first.position, self.center);
                let cursor_screen = self.canvas.dvec2_to_screen(cursor_world, self.center);
                let color = egui::Color32::from_rgba_premultiplied(160, 160, 170, 160);
                self.painter.line_segment(
                    [first_screen, cursor_screen],
                    egui::Stroke::new(2.0, color),
                );
                self.painter.circle_filled(first_screen, 5.0, color);
            }
            return;
        }

        // Point tool preview
        if active_tool == Tool::Point {
            if let Some((snap_pos, kind)) = point_preview {
                let screen = self.canvas.dvec2_to_screen(snap_pos, self.center);
                match kind {
                    PointPreviewKind::Existing => {
                        // Highlight existing point (blue ring)
                        self.painter.circle_stroke(
                            screen,
                            8.0,
                            egui::Stroke::new(2.0, egui::Color32::from_rgba_premultiplied(60, 160, 255, 180)),
                        );
                    }
                    PointPreviewKind::OnEdge => {
                        // Preview split-on-edge point (orange)
                        self.painter.circle_filled(
                            screen,
                            5.0,
                            egui::Color32::from_rgba_premultiplied(255, 180, 50, 180),
                        );
                        self.painter.circle_stroke(
                            screen,
                            5.0,
                            egui::Stroke::new(1.5, egui::Color32::from_rgba_premultiplied(255, 140, 0, 220)),
                        );
                    }
                    PointPreviewKind::New => {
                        // Preview new point (white with blue stroke)
                        self.painter.circle_filled(
                            screen,
                            5.0,
                            egui::Color32::from_rgba_premultiplied(255, 255, 255, 140),
                        );
                        self.painter.circle_stroke(
                            screen,
                            5.0,
                            egui::Stroke::new(1.5, egui::Color32::from_rgba_premultiplied(60, 160, 255, 180)),
                        );
                    }
                }
            }
            return;
        }

        let Some(cursor_world) = self.canvas.cursor_world_pos else {
            return;
        };
        let cursor_screen = self.canvas.dvec2_to_screen(cursor_world, self.center);

        let color = match active_tool {
            Tool::Cutout => egui::Color32::from_rgba_premultiplied(255, 100, 100, 160),
            Tool::Room => egui::Color32::from_rgba_premultiplied(70, 180, 130, 160),
            Tool::Wall => egui::Color32::from_rgba_premultiplied(180, 180, 180, 160),
            Tool::Door => egui::Color32::from_rgba_premultiplied(180, 120, 60, 160),
            Tool::Window => egui::Color32::from_rgba_premultiplied(80, 160, 220, 160),
            _ => return,
        };
        self.draw_polygon_preview(cursor_screen, tool_points, color);
    }

    fn draw_polygon_preview(
        &self,
        cursor_screen: egui::Pos2,
        point_ids: &[uuid::Uuid],
        color: egui::Color32,
    ) {
        if point_ids.is_empty() {
            return;
        }

        let screen_pts =
            polygon_screen_coords(point_ids, self.project, self.canvas, self.center);
        if screen_pts.is_empty() {
            return;
        }

        for i in 0..screen_pts.len().saturating_sub(1) {
            self.painter.line_segment(
                [screen_pts[i], screen_pts[i + 1]],
                egui::Stroke::new(2.0, color),
            );
        }

        if let Some(&last) = screen_pts.last() {
            self.painter
                .line_segment([last, cursor_screen], egui::Stroke::new(1.5, color));
        }

        for (i, sp) in screen_pts.iter().enumerate() {
            let r = if i == 0 { 6.0 } else { 4.0 };
            self.painter.circle_filled(*sp, r, color);
        }

        // Close indicator
        if screen_pts.len() >= 3 {
            let dist_to_first = ((cursor_screen.x - screen_pts[0].x).powi(2)
                + (cursor_screen.y - screen_pts[0].y).powi(2))
            .sqrt();
            if dist_to_first < 15.0 {
                self.painter
                    .circle_stroke(screen_pts[0], 10.0, egui::Stroke::new(2.0, color));
            }
        }
    }

    pub fn draw_empty_hint(&self, rect: egui::Rect, active_tool: Tool) {
        if self.project.points.is_empty() {
            let tool_hint = match active_tool {
                Tool::Select => "Режим выбора — кликните на объект",
                Tool::Point => "Кликните для размещения точки",
                Tool::Edge => "Кликните на две точки для создания ребра",
                Tool::Cutout => "Кликните 3+ точки для вырезания из комнаты",
                Tool::Room => "Сначала создайте точки (2), затем соберите контур",
                Tool::Wall => "Сначала создайте точки (2), затем соберите полигон",
                Tool::Door => "Сначала создайте точки (2), затем полигон двери",
                Tool::Window => "Сначала создайте точки (2), затем полигон окна",
                Tool::Label => "Кликните для размещения надписи",
            };
            self.painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tool_hint,
                egui::FontId::proportional(16.0),
                egui::Color32::from_rgb(120, 120, 120),
            );
        }
    }

    pub fn draw_status_bar(&self, rect: egui::Rect) {
        if let Some(pos) = self.canvas.cursor_world_pos {
            let zoom_pct = self.canvas.zoom * 200.0;
            let status = format!(
                "X: {:.0} мм  Y: {:.0} мм  |  Масштаб: {:.0}%",
                pos.x, pos.y, zoom_pct
            );
            self.painter.text(
                egui::pos2(rect.left() + 8.0, rect.bottom() - 20.0),
                egui::Align2::LEFT_CENTER,
                status,
                egui::FontId::monospace(12.0),
                egui::Color32::from_rgb(180, 180, 180),
            );
        }
    }
}
