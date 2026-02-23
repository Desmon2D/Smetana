use eframe::egui;

use crate::editor::{EditorTool, Selection, SnapType, WallToolState};
use crate::editor::wall_joints::compute_joints;
use crate::model::OpeningKind;
use super::{App, SECTION_COLORS};

/// Draw text centered at `center_pos`, rotated to follow the wall angle.
/// The angle is automatically flipped so text is never upside-down.
fn paint_rotated_text(
    painter: &egui::Painter,
    center_pos: egui::Pos2,
    text: String,
    font_id: egui::FontId,
    color: egui::Color32,
    wall_angle: f32,
) {
    // Normalize angle so text is never upside-down
    let angle = if wall_angle > std::f32::consts::FRAC_PI_2 {
        wall_angle - std::f32::consts::PI
    } else if wall_angle < -std::f32::consts::FRAC_PI_2 {
        wall_angle + std::f32::consts::PI
    } else {
        wall_angle
    };

    let galley = painter.layout_no_wrap(text, font_id, color);
    let w = galley.size().x;
    let h = galley.size().y;

    // TextShape rotates clockwise around `pos` (the upper-left corner).
    // egui's Rot2 applies: x' = c*x - s*y, y' = s*x + c*y
    // The local center at (w/2, h/2) after rotation lands at:
    //   offset_x = cos(θ)*w/2 - sin(θ)*h/2
    //   offset_y = sin(θ)*w/2 + cos(θ)*h/2
    // Set pos so that galley_pos + offset = center_pos.
    let (sin_a, cos_a) = angle.sin_cos();
    let offset_x = cos_a * (w / 2.0) - sin_a * (h / 2.0);
    let offset_y = sin_a * (w / 2.0) + cos_a * (h / 2.0);
    let pos = egui::pos2(center_pos.x - offset_x, center_pos.y - offset_y);

    let text_shape = egui::epaint::TextShape::new(pos, galley, color)
        .with_angle(angle);
    painter.add(text_shape);
}

impl App {
    pub(super) fn draw_walls(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let wall_fill = egui::Color32::from_rgb(140, 140, 145);
        let wall_outline = egui::Color32::from_rgb(40, 40, 42);
        let start_color = egui::Color32::from_rgb(60, 200, 80);
        let end_color = egui::Color32::from_rgb(230, 210, 50);


        let selected_id = match self.editor.selection {
            Selection::Wall(id) => Some(id),
            _ => None,
        };

        let (joint_map, hub_polygons) = compute_joints(&self.project.walls, &self.editor.canvas, center);

        // Blend a palette color with the wall gray base to produce a muted opaque fill
        let blend_color = |palette: (u8, u8, u8), factor: f32| -> egui::Color32 {
            let base = (140.0_f32, 140.0, 145.0);
            egui::Color32::from_rgb(
                (base.0 * (1.0 - factor) + palette.0 as f32 * factor) as u8,
                (base.1 * (1.0 - factor) + palette.1 as f32 * factor) as u8,
                (base.2 * (1.0 - factor) + palette.2 as f32 * factor) as u8,
            )
        };

        // Collect overlay elements to draw on top of hub polygons
        struct WallOverlay {
            is_selected: bool,
            corners: [egui::Pos2; 4],
            start_screen: egui::Pos2,
            end_screen: egui::Pos2,
            wall_angle: f32,
            len: f32,
            thickness_mm: f64,
        }

        let mut overlays: Vec<WallOverlay> = Vec::new();

        // Collect section label data for deferred drawing
        struct SectionLabel {
            pos: egui::Pos2,
            text: String,
            color: egui::Color32,
            wall_angle: f32,
        }
        let mut section_labels: Vec<SectionLabel> = Vec::new();

        // --- Pass 1: wall geometry (bodies, section fills, junction ticks) ---
        for wall in &self.project.walls {
            let is_selected = selected_id == Some(wall.id);

            let start_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.start.x as f32, wall.start.y as f32),
                center,
            );
            let end_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.end.x as f32, wall.end.y as f32),
                center,
            );

            let dx = end_screen.x - start_screen.x;
            let dy = end_screen.y - start_screen.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.1 {
                continue;
            }

            let half_thick_screen = (wall.thickness as f32 * self.editor.canvas.zoom) / 2.0;
            let nx = -dy / len * half_thick_screen;
            let ny = dx / len * half_thick_screen;

            let default_start_left = egui::pos2(start_screen.x + nx, start_screen.y + ny);
            let default_end_left = egui::pos2(end_screen.x + nx, end_screen.y + ny);
            let default_end_right = egui::pos2(end_screen.x - nx, end_screen.y - ny);
            let default_start_right = egui::pos2(start_screen.x - nx, start_screen.y - ny);

            let (start_left, start_right) = match joint_map.get(&(wall.id, false)) {
                Some(jv) => (jv.left, jv.right),
                None => (default_start_left, default_start_right),
            };
            let (end_left, end_right) = match joint_map.get(&(wall.id, true)) {
                Some(jv) => (jv.left, jv.right),
                None => (default_end_left, default_end_right),
            };

            let corners = [start_left, end_left, end_right, start_right];

            // Compute left side section count for global color offset
            let left_section_count = wall.left_side.junctions.len() + 1;

            // Section fill quads — each section is an opaque half-width polygon
            for (side_idx, (side_data, sign)) in [
                (&wall.left_side, 1.0_f32),
                (&wall.right_side, -1.0_f32),
            ].iter().enumerate() {
                let mut boundaries = vec![0.0_f32];
                for j in &side_data.junctions {
                    boundaries.push(j.t as f32);
                }
                boundaries.push(1.0);

                let color_offset = if side_idx == 0 { 0 } else { left_section_count };

                // Mitered outer corners for this side
                let (side_start, side_end) = if *sign > 0.0 {
                    (start_left, end_left)
                } else {
                    (start_right, end_right)
                };

                for i in 0..boundaries.len() - 1 {
                    let t0 = boundaries[i];
                    let t1 = boundaries[i + 1];

                    let section_color = if is_selected {
                        let global_idx = color_offset + i;
                        let color_idx = global_idx % SECTION_COLORS.len();
                        blend_color(SECTION_COLORS[color_idx], 0.35)
                    } else {
                        wall_fill
                    };

                    // Centerline points
                    let p0_center = egui::pos2(
                        start_screen.x + dx * t0,
                        start_screen.y + dy * t0,
                    );
                    let p1_center = egui::pos2(
                        start_screen.x + dx * t1,
                        start_screen.y + dy * t1,
                    );

                    // Outer edge: use mitered corners at wall endpoints, straight normal elsewhere
                    let p0_edge = if t0 == 0.0 {
                        side_start
                    } else {
                        egui::pos2(p0_center.x + nx * *sign, p0_center.y + ny * *sign)
                    };
                    let p1_edge = if t1 == 1.0 {
                        side_end
                    } else {
                        egui::pos2(p1_center.x + nx * *sign, p1_center.y + ny * *sign)
                    };

                    // Ensure consistent CW winding for both sides
                    let quad = if *sign > 0.0 {
                        vec![p0_center, p1_center, p1_edge, p0_edge]
                    } else {
                        vec![p0_center, p0_edge, p1_edge, p1_center]
                    };
                    painter.add(egui::Shape::convex_polygon(
                        quad,
                        section_color,
                        egui::Stroke::NONE,
                    ));
                }

                // Junction tick marks — only when selected
                if is_selected {
                    for j in &side_data.junctions {
                        let jt = j.t as f32;
                        let jx = start_screen.x + dx * jt;
                        let jy = start_screen.y + dy * jt;
                        let j_center = egui::pos2(jx, jy);
                        let j_edge = egui::pos2(jx + nx * *sign, jy + ny * *sign);
                        painter.line_segment(
                            [j_center, j_edge],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 200, 60)),
                        );
                    }
                }
            }

            // Wall outline (mitered perimeter) on top of section fills
            painter.add(egui::Shape::closed_line(
                corners.to_vec(),
                egui::Stroke::new(1.0, wall_outline),
            ));

            let wall_angle = dy.atan2(dx);

            // Collect overlay data for this wall
            overlays.push(WallOverlay {
                is_selected,
                corners,
                start_screen,
                end_screen,
                wall_angle,
                len,
                thickness_mm: wall.thickness,
            });

            // Collect section labels (always shown; colored when selected)
            let unselected_label_color = egui::Color32::from_rgb(160, 160, 165);
            for (side_idx, (side_data, sign)) in [
                (&wall.left_side, 1.0_f32),
                (&wall.right_side, -1.0_f32),
            ].iter().enumerate() {
                let mut boundaries = vec![0.0_f32];
                for j in &side_data.junctions {
                    boundaries.push(j.t as f32);
                }
                boundaries.push(1.0);

                let color_offset = if side_idx == 0 { 0 } else { left_section_count };

                for (i, section) in side_data.sections.iter().enumerate() {
                    if i >= boundaries.len() - 1 {
                        break;
                    }
                    let t0 = boundaries[i];
                    let t1 = boundaries[i + 1];
                    let t_mid = (t0 + t1) / 2.0;

                    let mid_x = start_screen.x + dx * t_mid + nx * sign * 1.6;
                    let mid_y = start_screen.y + dy * t_mid + ny * sign * 1.6;

                    let label_color = if is_selected {
                        let global_idx = color_offset + i;
                        let color_idx = global_idx % SECTION_COLORS.len();
                        let (cr, cg, cb) = SECTION_COLORS[color_idx];
                        egui::Color32::from_rgb(cr, cg, cb)
                    } else {
                        unselected_label_color
                    };

                    let length_mm = section.length;
                    let area_m2 = section.gross_area() / 1_000_000.0;
                    section_labels.push(SectionLabel {
                        pos: egui::pos2(mid_x, mid_y),
                        text: format!("{:.0} мм - {:.2} м²", length_mm, area_m2),
                        color: label_color,
                        wall_angle,
                    });
                }
            }
        }

        // --- Draw hub polygons (joint fills) ---
        for hub in &hub_polygons {
            if hub.vertices.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    hub.vertices.clone(),
                    hub.fill,
                    egui::Stroke::new(1.0, wall_outline),
                ));
            }
        }

        // --- Pass 2: overlays on top (selection outlines, endpoint circles, text labels) ---
        for ov in &overlays {
            if ov.is_selected {
                let sel_outline = egui::Color32::from_rgb(60, 160, 255);
                painter.add(egui::Shape::closed_line(
                    ov.corners.to_vec(),
                    egui::Stroke::new(2.5, sel_outline),
                ));
                painter.circle_filled(ov.start_screen, 4.0, start_color);
                painter.circle_filled(ov.end_screen, 4.0, end_color);
            }

            // Wall thickness label at center
            if ov.len > 20.0 {
                let mid = egui::pos2(
                    (ov.start_screen.x + ov.end_screen.x) / 2.0,
                    (ov.start_screen.y + ov.end_screen.y) / 2.0,
                );
                let label = format!("{:.0}", ov.thickness_mm);
                paint_rotated_text(
                    painter,
                    mid,
                    label,
                    egui::FontId::proportional(10.0 * self.label_scale),
                    egui::Color32::BLACK,
                    ov.wall_angle,
                );
            }
        }

        // Section dimension labels
        for sl in &section_labels {
            paint_rotated_text(
                painter,
                sl.pos,
                sl.text.clone(),
                egui::FontId::proportional(9.0 * self.label_scale),
                sl.color,
                sl.wall_angle,
            );
        }
    }

    pub(super) fn draw_wall_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let preview_color = egui::Color32::from_rgba_premultiplied(100, 180, 255, 180);
        let start_marker_color = egui::Color32::from_rgb(100, 180, 255);
        let chain_start_color = egui::Color32::from_rgb(60, 220, 120);

        if let Some(chain_start) = self.editor.wall_tool.chain_start {
            if let WallToolState::Drawing { start } = &self.editor.wall_tool.state {
                if start.distance(chain_start) > 1.0 {
                    let cs_screen = self.editor.canvas.world_to_screen(
                        egui::pos2(chain_start.x as f32, chain_start.y as f32),
                        center,
                    );
                    painter.circle_filled(cs_screen, 6.0, chain_start_color);
                }
            }
        }

        if let WallToolState::Drawing { start } = &self.editor.wall_tool.state {
            let start_screen = self.editor.canvas.world_to_screen(
                egui::pos2(start.x as f32, start.y as f32),
                center,
            );

            painter.circle_filled(start_screen, 5.0, start_marker_color);

            if let Some(end) = self.editor.wall_tool.preview_end {
                let end_screen = self.editor.canvas.world_to_screen(
                    egui::pos2(end.x as f32, end.y as f32),
                    center,
                );
                painter.line_segment(
                    [start_screen, end_screen],
                    egui::Stroke::new(2.0, preview_color),
                );

                let length_mm = start.distance(end);
                if length_mm > 1.0 {
                    let pdx = end_screen.x - start_screen.x;
                    let pdy = end_screen.y - start_screen.y;
                    let preview_angle = pdy.atan2(pdx);

                    // Offset label perpendicular to the wall (above the line)
                    let plen = (pdx * pdx + pdy * pdy).sqrt().max(0.001);
                    let perp_x = -pdy / plen * 12.0;
                    let perp_y = pdx / plen * 12.0;
                    let mid = egui::pos2(
                        (start_screen.x + end_screen.x) / 2.0 + perp_x,
                        (start_screen.y + end_screen.y) / 2.0 + perp_y,
                    );
                    let label = if length_mm >= 1000.0 {
                        format!("{:.2} м", length_mm / 1000.0)
                    } else {
                        format!("{:.0} мм", length_mm)
                    };
                    paint_rotated_text(
                        painter,
                        mid,
                        label,
                        egui::FontId::proportional(12.0 * self.label_scale),
                        preview_color,
                        preview_angle,
                    );
                }
            }
        }

        if let Some(end) = self.editor.wall_tool.preview_end {
            let end_screen = self.editor.canvas.world_to_screen(
                egui::pos2(end.x as f32, end.y as f32),
                center,
            );

            // Snap indicator: colored ring based on snap type
            let (indicator_color, radius) = match &self.editor.wall_tool.last_snap {
                Some(snap) => match &snap.snap_type {
                    SnapType::Vertex => (egui::Color32::from_rgb(60, 200, 80), 6.0),
                    SnapType::WallEdge { .. } => (egui::Color32::from_rgb(230, 210, 50), 6.0),
                    SnapType::Grid => (egui::Color32::from_rgb(180, 180, 180), 4.0),
                    SnapType::None => (egui::Color32::TRANSPARENT, 0.0),
                },
                None => (egui::Color32::TRANSPARENT, 0.0),
            };
            if indicator_color != egui::Color32::TRANSPARENT {
                painter.circle_stroke(
                    end_screen,
                    radius,
                    egui::Stroke::new(2.0, indicator_color),
                );
            }
        }
    }

    pub(super) fn draw_openings(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let canvas_bg = egui::Color32::from_rgb(45, 45, 48);

        let selected_opening_id = match self.editor.selection {
            Selection::Opening(id) => Some(id),
            _ => None,
        };

        for opening in &self.project.openings {
            let wall = match opening.wall_id {
                Some(wid) => match self.project.walls.iter().find(|w| w.id == wid) {
                    Some(w) => w,
                    None => continue,
                },
                None => {
                    let pos = match self.editor.orphan_positions.get(&opening.id) {
                        Some(p) => *p,
                        None => continue,
                    };
                    let screen_pos = self.editor.canvas.world_to_screen(
                        egui::pos2(pos.x as f32, pos.y as f32), center,
                    );
                    let w_screen = opening.kind.width() as f32 * self.editor.canvas.zoom;
                    let h_screen = 6.0_f32.max(opening.kind.height() as f32 * self.editor.canvas.zoom * 0.05);
                    let r = egui::Rect::from_center_size(
                        screen_pos,
                        egui::vec2(w_screen, h_screen),
                    );
                    let red = egui::Color32::from_rgb(220, 50, 50);
                    let is_selected = selected_opening_id == Some(opening.id);
                    let stroke_w = if is_selected { 3.0 } else { 2.0 };
                    painter.rect_stroke(r, 0.0, egui::Stroke::new(stroke_w, red), egui::StrokeKind::Outside);
                    let label = match &opening.kind {
                        OpeningKind::Door { .. } => "⚠ Дверь",
                        OpeningKind::Window { .. } => "⚠ Окно",
                    };
                    painter.text(
                        egui::pos2(screen_pos.x, r.top() - 4.0),
                        egui::Align2::CENTER_BOTTOM,
                        label,
                        egui::FontId::proportional(12.0 * self.label_scale),
                        red,
                    );
                    continue;
                }
            };

            let wall_len = wall.length();
            if wall_len < 1.0 {
                continue;
            }

            let opening_width = opening.kind.width();
            let half_w = opening_width / 2.0;
            let t_left = ((opening.offset_along_wall - half_w) / wall_len).clamp(0.0, 1.0);
            let t_right = ((opening.offset_along_wall + half_w) / wall_len).clamp(0.0, 1.0);

            let lerp_wall = |t: f64| -> egui::Pos2 {
                let wx = wall.start.x + (wall.end.x - wall.start.x) * t;
                let wy = wall.start.y + (wall.end.y - wall.start.y) * t;
                self.editor
                    .canvas
                    .world_to_screen(egui::pos2(wx as f32, wy as f32), center)
            };

            let p_left = lerp_wall(t_left);
            let p_right = lerp_wall(t_right);

            let start_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.start.x as f32, wall.start.y as f32),
                center,
            );
            let end_screen = self.editor.canvas.world_to_screen(
                egui::pos2(wall.end.x as f32, wall.end.y as f32),
                center,
            );
            let dx = end_screen.x - start_screen.x;
            let dy = end_screen.y - start_screen.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.1 {
                continue;
            }

            let half_thick = (wall.thickness as f32 * self.editor.canvas.zoom) / 2.0;
            let nx = -dy / len * half_thick;
            let ny = dx / len * half_thick;

            let is_selected = selected_opening_id == Some(opening.id);

            let gap_corners = [
                egui::pos2(p_left.x + nx * 1.1, p_left.y + ny * 1.1),
                egui::pos2(p_right.x + nx * 1.1, p_right.y + ny * 1.1),
                egui::pos2(p_right.x - nx * 1.1, p_right.y - ny * 1.1),
                egui::pos2(p_left.x - nx * 1.1, p_left.y - ny * 1.1),
            ];
            painter.add(egui::Shape::convex_polygon(
                gap_corners.to_vec(),
                canvas_bg,
                egui::Stroke::NONE,
            ));

            match &opening.kind {
                OpeningKind::Door { .. } => {
                    let color = if is_selected {
                        egui::Color32::from_rgb(240, 180, 80)
                    } else {
                        egui::Color32::from_rgb(180, 120, 60)
                    };
                    let stroke_w = if is_selected { 2.0 } else { 1.5 };

                    painter.line_segment(
                        [p_left, p_right],
                        egui::Stroke::new(stroke_w, color),
                    );

                    let arc_r = ((p_right.x - p_left.x).powi(2)
                        + (p_right.y - p_left.y).powi(2))
                    .sqrt();
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
                            pts.push(egui::pos2(
                                p_left.x + d_x * arc_r,
                                p_left.y + d_y * arc_r,
                            ));
                        }
                        for i in 0..n_seg {
                            painter.line_segment(
                                [pts[i], pts[i + 1]],
                                egui::Stroke::new(stroke_w, color),
                            );
                        }
                    }
                }
                OpeningKind::Window { .. } => {
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
            }
        }
    }

    pub(super) fn draw_rooms(&self, painter: &egui::Painter, rect: egui::Rect) {
        use crate::editor::room_metrics::compute_room_metrics;

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

        for (i, room) in self.project.rooms.iter().enumerate() {
            let metrics = match compute_room_metrics(room, &self.project.walls) {
                Some(m) => m,
                None => continue,
            };

            if metrics.inner_polygon.len() < 3 {
                continue;
            }

            let (r, g, b) = ROOM_COLORS[i % ROOM_COLORS.len()];
            let fill = egui::Color32::from_rgba_unmultiplied(r, g, b, 40);

            let screen_pts: Vec<egui::Pos2> = metrics
                .inner_polygon
                .iter()
                .map(|p| {
                    self.editor
                        .canvas
                        .world_to_screen(egui::pos2(p.x as f32, p.y as f32), center)
                })
                .collect();

            let triangles = crate::editor::triangulation::triangulate(&screen_pts);
            for tri in &triangles {
                let tri_pts = vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]];
                painter.add(egui::Shape::convex_polygon(tri_pts, fill, egui::Stroke::NONE));
            }
            painter.add(egui::Shape::closed_line(screen_pts.clone(), egui::Stroke::new(1.0, fill)));

            let cx: f32 = screen_pts.iter().map(|p| p.x).sum::<f32>() / screen_pts.len() as f32;
            let cy: f32 = screen_pts.iter().map(|p| p.y).sum::<f32>() / screen_pts.len() as f32;

            let label_color = egui::Color32::from_rgb(r, g, b);
            painter.text(
                egui::pos2(cx, cy),
                egui::Align2::CENTER_CENTER,
                &room.name,
                egui::FontId::proportional(13.0 * self.label_scale),
                label_color,
            );

            let area_m2 = metrics.net_area / 1_000_000.0;
            painter.text(
                egui::pos2(cx, cy + 16.0 * self.label_scale),
                egui::Align2::CENTER_CENTER,
                format!("{:.1} м²", area_m2),
                egui::FontId::proportional(11.0 * self.label_scale),
                label_color,
            );
        }
    }

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
            let screen_pos = self.editor.canvas.world_to_screen(
                egui::pos2(label.position.x as f32, label.position.y as f32),
                center,
            );
            let is_selected = selected_label_id == Some(label.id);
            let color = if is_selected { selected_color } else { normal_color };
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

    pub(super) fn draw_opening_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let wall_id = match self.editor.opening_tool.hover_wall_id {
            Some(id) => id,
            None => return,
        };
        let wall = match self.project.walls.iter().find(|w| w.id == wall_id) {
            Some(w) => w,
            None => return,
        };

        let opening_width = if self.editor.active_tool == EditorTool::Door {
            900.0
        } else {
            1200.0
        };

        let offset = self.editor.opening_tool.hover_offset;
        let wall_len = wall.length();
        if wall_len < 1.0 {
            return;
        }

        let half_w = opening_width / 2.0;
        let t_center = offset / wall_len;
        let t_left = ((offset - half_w) / wall_len).clamp(0.0, 1.0);
        let t_right = ((offset + half_w) / wall_len).clamp(0.0, 1.0);

        let lerp_wall = |t: f64| -> egui::Pos2 {
            let wx = wall.start.x + (wall.end.x - wall.start.x) * t;
            let wy = wall.start.y + (wall.end.y - wall.start.y) * t;
            self.editor.canvas.world_to_screen(egui::pos2(wx as f32, wy as f32), center)
        };

        let p_left = lerp_wall(t_left);
        let p_right = lerp_wall(t_right);
        let p_center = lerp_wall(t_center);

        let dx = p_right.x - p_left.x;
        let dy = p_right.y - p_left.y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len < 0.5 {
            return;
        }

        let half_thick_screen = (wall.thickness as f32 * self.editor.canvas.zoom) / 2.0;
        let nx = -dy / seg_len * half_thick_screen;
        let ny = dx / seg_len * half_thick_screen;

        let preview_color = if self.editor.active_tool == EditorTool::Door {
            egui::Color32::from_rgba_premultiplied(180, 120, 60, 160)
        } else {
            egui::Color32::from_rgba_premultiplied(80, 160, 220, 160)
        };

        let corners = [
            egui::pos2(p_left.x + nx, p_left.y + ny),
            egui::pos2(p_right.x + nx, p_right.y + ny),
            egui::pos2(p_right.x - nx, p_right.y - ny),
            egui::pos2(p_left.x - nx, p_left.y - ny),
        ];

        painter.add(egui::Shape::convex_polygon(
            corners.to_vec(),
            preview_color,
            egui::Stroke::new(2.0, preview_color),
        ));

        let label = if self.editor.active_tool == EditorTool::Door {
            "Дверь"
        } else {
            "Окно"
        };
        painter.text(
            egui::pos2(p_center.x, p_center.y - half_thick_screen - 10.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            egui::FontId::proportional(11.0 * self.label_scale),
            preview_color,
        );
    }
}
