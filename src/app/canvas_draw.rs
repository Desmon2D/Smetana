use eframe::egui;
use glam::DVec2;

use crate::editor::{Canvas, EditorTool, Selection, SnapType, WallToolState};
use crate::editor::wall_joints::compute_joints;
use crate::model::{OpeningKind, Wall};
use super::{App, SECTION_COLORS};

/// Convert a world-space DVec2 (mm) to screen-space Pos2 via the canvas.
fn world_to_screen(canvas: &Canvas, center: egui::Pos2, p: DVec2) -> egui::Pos2 {
    canvas.world_to_screen(egui::pos2(p.x as f32, p.y as f32), center)
}

// ---------------------------------------------------------------------------
// WallScreenGeometry — shared screen-space representation of a wall
// ---------------------------------------------------------------------------

/// Pre-computed screen-space geometry for a wall, used by draw_walls,
/// draw_openings, and draw_opening_preview to avoid duplicating the same
/// world-to-screen and normal calculations.
struct WallScreenGeometry {
    start: egui::Pos2,
    end: egui::Pos2,
    dx: f32,
    dy: f32,
    len: f32,
    half_thick: f32,
    nx: f32,
    ny: f32,
    angle: f32,
}

impl WallScreenGeometry {
    /// Build screen geometry for a wall. Returns `None` if the wall is too
    /// short to render (screen length < 0.1 px).
    fn from_wall(wall: &Wall, canvas: &Canvas, center: egui::Pos2) -> Option<Self> {
        let start = canvas.world_to_screen(
            egui::pos2(wall.start.x as f32, wall.start.y as f32),
            center,
        );
        let end = canvas.world_to_screen(
            egui::pos2(wall.end.x as f32, wall.end.y as f32),
            center,
        );
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.1 {
            return None;
        }
        let half_thick = (wall.thickness as f32 * canvas.zoom) / 2.0;
        let nx = -dy / len * half_thick;
        let ny = dx / len * half_thick;
        let angle = dy.atan2(dx);
        Some(Self { start, end, dx, dy, len, half_thick, nx, ny, angle })
    }

    /// Interpolate a point along the wall centerline at parameter `t` in [0, 1].
    fn lerp(&self, t: f32) -> egui::Pos2 {
        egui::pos2(
            self.start.x + self.dx * t,
            self.start.y + self.dy * t,
        )
    }

    /// Point on the left edge (positive normal side) at parameter `t`.
    fn left_at(&self, t: f32) -> egui::Pos2 {
        let c = self.lerp(t);
        egui::pos2(c.x + self.nx, c.y + self.ny)
    }

    /// Point on the right edge (negative normal side) at parameter `t`.
    fn right_at(&self, t: f32) -> egui::Pos2 {
        let c = self.lerp(t);
        egui::pos2(c.x - self.nx, c.y - self.ny)
    }
}

// ---------------------------------------------------------------------------
// WallOverlay / SectionLabel — module-level data collected during pass 1
// ---------------------------------------------------------------------------

struct WallOverlay {
    is_selected: bool,
    corners: [egui::Pos2; 4],
    start_screen: egui::Pos2,
    end_screen: egui::Pos2,
    wall_angle: f32,
    len: f32,
    thickness_mm: f64,
}

struct SectionLabel {
    pos: egui::Pos2,
    text: String,
    color: egui::Color32,
    wall_angle: f32,
}

// ---------------------------------------------------------------------------
// Helper: compute section boundary t-values for a wall side
// ---------------------------------------------------------------------------

fn wall_section_boundaries(side: &crate::model::SideData) -> Vec<f32> {
    let mut boundaries = Vec::with_capacity(side.junctions.len() + 2);
    boundaries.push(0.0_f32);
    for j in &side.junctions {
        boundaries.push(j.t as f32);
    }
    boundaries.push(1.0);
    boundaries
}

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

    let (sin_a, cos_a) = angle.sin_cos();
    let offset_x = cos_a * (w / 2.0) - sin_a * (h / 2.0);
    let offset_y = sin_a * (w / 2.0) + cos_a * (h / 2.0);
    let pos = egui::pos2(center_pos.x - offset_x, center_pos.y - offset_y);

    let text_shape = egui::epaint::TextShape::new(pos, galley, color)
        .with_angle(angle);
    painter.add(text_shape);
}

// ---------------------------------------------------------------------------
// Door / Window symbol rendering (extracted from draw_openings)
// ---------------------------------------------------------------------------

/// Draw a door symbol: a straight line from p_left to p_right plus a 90-degree
/// swing arc.
fn draw_door_symbol(
    painter: &egui::Painter,
    p_left: egui::Pos2,
    p_right: egui::Pos2,
    _nx: f32,
    _ny: f32,
    is_selected: bool,
) {
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

/// Draw a window symbol: two parallel lines offset from the centerline plus
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
    // draw_walls — two-pass: geometry then overlays
    // -----------------------------------------------------------------------

    pub(super) fn draw_walls(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let (joint_map, hub_polygons) =
            compute_joints(&self.project.walls);

        let (overlays, section_labels) =
            self.draw_wall_geometry(painter, rect, &joint_map);

        // Hub polygons (joint fills) between passes — use triangulation
        // instead of convex_polygon to handle non-convex hub shapes correctly.
        let wall_outline = egui::Color32::from_rgb(40, 40, 42);
        for hub in &hub_polygons {
            if hub.vertices.len() >= 3 {
                let screen_verts: Vec<egui::Pos2> = hub.vertices
                    .iter()
                    .map(|p| world_to_screen(&self.editor.canvas, center, *p))
                    .collect();
                let triangles = triangulate(&screen_verts);
                for tri in &triangles {
                    let tri_pts = vec![screen_verts[tri[0]], screen_verts[tri[1]], screen_verts[tri[2]]];
                    painter.add(egui::Shape::convex_polygon(tri_pts, hub.fill, egui::Stroke::NONE));
                }
                painter.add(egui::Shape::closed_line(
                    screen_verts,
                    egui::Stroke::new(1.0, wall_outline),
                ));
            }
        }

        self.draw_wall_overlays(painter, &overlays, &section_labels);
    }

    /// Pass 1: render wall bodies, section fills, junction ticks and outlines.
    /// Returns overlay data and section labels for deferred rendering in pass 2.
    fn draw_wall_geometry(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        joint_map: &std::collections::HashMap<(uuid::Uuid, bool), crate::editor::wall_joints::JointVertices>,
    ) -> (Vec<WallOverlay>, Vec<SectionLabel>) {
        let center = rect.center();
        let wall_fill = egui::Color32::from_rgb(140, 140, 145);
        let wall_outline = egui::Color32::from_rgb(40, 40, 42);

        let selected_id = match self.editor.selection {
            Selection::Wall(id) => Some(id),
            _ => None,
        };

        let blend_color = |palette: (u8, u8, u8), factor: f32| -> egui::Color32 {
            let base = (140.0_f32, 140.0, 145.0);
            egui::Color32::from_rgb(
                (base.0 * (1.0 - factor) + palette.0 as f32 * factor) as u8,
                (base.1 * (1.0 - factor) + palette.1 as f32 * factor) as u8,
                (base.2 * (1.0 - factor) + palette.2 as f32 * factor) as u8,
            )
        };

        let mut overlays: Vec<WallOverlay> = Vec::new();
        let mut section_labels: Vec<SectionLabel> = Vec::new();

        for wall in &self.project.walls {
            let geo = match WallScreenGeometry::from_wall(wall, &self.editor.canvas, center) {
                Some(g) => g,
                None => continue,
            };
            let is_selected = selected_id == Some(wall.id);

            let default_start_left = geo.left_at(0.0);
            let default_end_left = geo.left_at(1.0);
            let default_end_right = geo.right_at(1.0);
            let default_start_right = geo.right_at(0.0);

            let (start_left, start_right) = match joint_map.get(&(wall.id, false)) {
                Some(jv) => (
                    world_to_screen(&self.editor.canvas, center, jv.left),
                    world_to_screen(&self.editor.canvas, center, jv.right),
                ),
                None => (default_start_left, default_start_right),
            };
            let (end_left, end_right) = match joint_map.get(&(wall.id, true)) {
                Some(jv) => (
                    world_to_screen(&self.editor.canvas, center, jv.left),
                    world_to_screen(&self.editor.canvas, center, jv.right),
                ),
                None => (default_end_left, default_end_right),
            };

            let corners = [start_left, end_left, end_right, start_right];

            let left_section_count = wall.left_side.junctions.len() + 1;

            // Section fill quads
            for (side_idx, (side_data, sign)) in [
                (&wall.left_side, 1.0_f32),
                (&wall.right_side, -1.0_f32),
            ].iter().enumerate() {
                let boundaries = wall_section_boundaries(side_data);
                let color_offset = if side_idx == 0 { 0 } else { left_section_count };

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

                    let p0_center = geo.lerp(t0);
                    let p1_center = geo.lerp(t1);

                    let p0_edge = if t0 == 0.0 {
                        side_start
                    } else {
                        egui::pos2(
                            p0_center.x + geo.nx * *sign,
                            p0_center.y + geo.ny * *sign,
                        )
                    };
                    let p1_edge = if t1 == 1.0 {
                        side_end
                    } else {
                        egui::pos2(
                            p1_center.x + geo.nx * *sign,
                            p1_center.y + geo.ny * *sign,
                        )
                    };

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
                        let j_center = geo.lerp(jt);
                        let j_edge = egui::pos2(
                            j_center.x + geo.nx * *sign,
                            j_center.y + geo.ny * *sign,
                        );
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

            // Collect overlay data
            overlays.push(WallOverlay {
                is_selected,
                corners,
                start_screen: geo.start,
                end_screen: geo.end,
                wall_angle: geo.angle,
                len: geo.len,
                thickness_mm: wall.thickness,
            });

            // Collect section labels (always shown; colored when selected)
            let unselected_label_color = egui::Color32::from_rgb(160, 160, 165);
            for (side_idx, (side_data, sign)) in [
                (&wall.left_side, 1.0_f32),
                (&wall.right_side, -1.0_f32),
            ].iter().enumerate() {
                let boundaries = wall_section_boundaries(side_data);
                let color_offset = if side_idx == 0 { 0 } else { left_section_count };

                for (i, section) in side_data.sections.iter().enumerate() {
                    if i >= boundaries.len() - 1 {
                        break;
                    }
                    let t_mid = (boundaries[i] + boundaries[i + 1]) / 2.0;

                    let mid_x = geo.start.x + geo.dx * t_mid + geo.nx * sign * 1.6;
                    let mid_y = geo.start.y + geo.dy * t_mid + geo.ny * sign * 1.6;

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
                        wall_angle: geo.angle,
                    });
                }
            }
        }

        (overlays, section_labels)
    }

    /// Pass 2: draw selection outlines, endpoint circles, and text labels
    /// on top of all geometry and hub polygons.
    fn draw_wall_overlays(
        &self,
        painter: &egui::Painter,
        overlays: &[WallOverlay],
        section_labels: &[SectionLabel],
    ) {
        let start_color = egui::Color32::from_rgb(60, 200, 80);
        let end_color = egui::Color32::from_rgb(230, 210, 50);

        for ov in overlays {
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
        for sl in section_labels {
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

    // -----------------------------------------------------------------------
    // draw_wall_preview
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // draw_openings — thin dispatch loop
    // -----------------------------------------------------------------------

    pub(super) fn draw_openings(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let selected_opening_id = match self.editor.selection {
            Selection::Opening(id) => Some(id),
            _ => None,
        };

        for opening in &self.project.openings {
            match opening.wall_id {
                Some(wid) => {
                    let wall = match self.project.wall(wid) {
                        Some(w) => w,
                        None => continue,
                    };
                    let is_selected = selected_opening_id == Some(opening.id);
                    self.draw_attached_opening(painter, center, opening, wall, is_selected);
                }
                None => {
                    let is_selected = selected_opening_id == Some(opening.id);
                    self.draw_orphaned_opening(painter, center, opening, is_selected);
                }
            }
        }
    }

    /// Draw an opening that is attached to a wall.
    fn draw_attached_opening(
        &self,
        painter: &egui::Painter,
        center: egui::Pos2,
        opening: &crate::model::Opening,
        wall: &Wall,
        is_selected: bool,
    ) {
        let geo = match WallScreenGeometry::from_wall(wall, &self.editor.canvas, center) {
            Some(g) => g,
            None => return,
        };

        let wall_len = wall.length();
        if wall_len < 1.0 {
            return;
        }

        let opening_width = opening.kind.width();
        let half_w = opening_width / 2.0;
        let t_left = ((opening.offset_along_wall - half_w) / wall_len).clamp(0.0, 1.0) as f32;
        let t_right = ((opening.offset_along_wall + half_w) / wall_len).clamp(0.0, 1.0) as f32;

        let p_left = geo.lerp(t_left);
        let p_right = geo.lerp(t_right);

        let canvas_bg = egui::Color32::from_rgb(45, 45, 48);

        // Gap cutout
        let gap_corners = [
            egui::pos2(p_left.x + geo.nx * 1.1, p_left.y + geo.ny * 1.1),
            egui::pos2(p_right.x + geo.nx * 1.1, p_right.y + geo.ny * 1.1),
            egui::pos2(p_right.x - geo.nx * 1.1, p_right.y - geo.ny * 1.1),
            egui::pos2(p_left.x - geo.nx * 1.1, p_left.y - geo.ny * 1.1),
        ];
        painter.add(egui::Shape::convex_polygon(
            gap_corners.to_vec(),
            canvas_bg,
            egui::Stroke::NONE,
        ));

        // Symbol
        match &opening.kind {
            OpeningKind::Door { .. } => {
                draw_door_symbol(painter, p_left, p_right, geo.nx, geo.ny, is_selected);
            }
            OpeningKind::Window { .. } => {
                draw_window_symbol(painter, p_left, p_right, geo.nx, geo.ny, is_selected);
            }
        }
    }

    /// Draw an orphaned opening (not attached to any wall).
    fn draw_orphaned_opening(
        &self,
        painter: &egui::Painter,
        center: egui::Pos2,
        opening: &crate::model::Opening,
        is_selected: bool,
    ) {
        let pos = match self.editor.orphan_positions.get(&opening.id) {
            Some(p) => *p,
            None => return,
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
        let stroke_w = if is_selected { 3.0 } else { 2.0 };
        painter.rect_stroke(r, 0.0, egui::Stroke::new(stroke_w, red), egui::StrokeKind::Outside);
        let label = match &opening.kind {
            OpeningKind::Door { .. } => "\u{26a0} Дверь",
            OpeningKind::Window { .. } => "\u{26a0} Окно",
        };
        painter.text(
            egui::pos2(screen_pos.x, r.top() - 4.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            egui::FontId::proportional(12.0 * self.label_scale),
            red,
        );
    }

    // -----------------------------------------------------------------------
    // draw_rooms
    // -----------------------------------------------------------------------

    pub(super) fn draw_rooms(&self, painter: &egui::Painter, rect: egui::Rect) {
        use crate::model::room_metrics::{compute_room_metrics, point_in_polygon, RoomMetrics};

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

        // Pass 1: compute metrics for all rooms
        let room_metrics: Vec<Option<RoomMetrics>> = self.project.rooms.iter()
            .map(|room| compute_room_metrics(room, &self.project.walls))
            .collect();

        // Pass 2: detect nesting — room j is nested inside room i if j's
        // centroid lies inside i's inner polygon.
        let mut nested_in: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();

        for (i, outer_metrics) in room_metrics.iter().enumerate() {
            let outer = match outer_metrics {
                Some(m) if m.inner_polygon.len() >= 3 => m,
                _ => continue,
            };
            for (j, inner_metrics) in room_metrics.iter().enumerate() {
                if i == j {
                    continue;
                }
                let inner = match inner_metrics {
                    Some(m) if m.inner_polygon.len() >= 3 => m,
                    _ => continue,
                };
                // Use centroid of inner room
                let centroid = {
                    let sum = inner.inner_polygon.iter().fold(
                        glam::DVec2::ZERO,
                        |acc, p| acc + *p,
                    );
                    sum / inner.inner_polygon.len() as f64
                };
                if point_in_polygon(centroid, &outer.inner_polygon) {
                    // j is nested inside i — but only if j's area < i's area
                    if inner.net_area < outer.net_area {
                        nested_in.entry(i).or_default().push(j);
                    }
                }
            }
        }

        // Pass 3: render rooms
        for (i, room) in self.project.rooms.iter().enumerate() {
            let metrics = match &room_metrics[i] {
                Some(m) if m.inner_polygon.len() >= 3 => m,
                _ => continue,
            };

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

            // Triangulate with holes if this room contains nested rooms
            if let Some(nested_indices) = nested_in.get(&i) {
                // Build coords with hole polygons
                let mut coords: Vec<f32> = screen_pts.iter().flat_map(|p| [p.x, p.y]).collect();
                let mut hole_indices = Vec::new();

                for &nested_idx in nested_indices {
                    let nested_metrics = match &room_metrics[nested_idx] {
                        Some(m) if m.inner_polygon.len() >= 3 => m,
                        _ => continue,
                    };
                    hole_indices.push(coords.len() / 2);
                    // Reverse winding for hole
                    for p in nested_metrics.inner_polygon.iter().rev() {
                        let sp = self.editor.canvas.world_to_screen(
                            egui::pos2(p.x as f32, p.y as f32),
                            center,
                        );
                        coords.extend([sp.x, sp.y]);
                    }
                }

                let all_pts: Vec<egui::Pos2> = coords.chunks(2)
                    .map(|c| egui::pos2(c[0], c[1]))
                    .collect();
                let indices = earcutr::earcut(&coords, &hole_indices, 2).unwrap_or_default();
                for tri in indices.chunks(3) {
                    if tri.len() == 3 {
                        let tri_pts = vec![all_pts[tri[0]], all_pts[tri[1]], all_pts[tri[2]]];
                        painter.add(egui::Shape::convex_polygon(tri_pts, fill, egui::Stroke::NONE));
                    }
                }
            } else {
                let triangles = triangulate(&screen_pts);
                for tri in &triangles {
                    let tri_pts = vec![screen_pts[tri[0]], screen_pts[tri[1]], screen_pts[tri[2]]];
                    painter.add(egui::Shape::convex_polygon(tri_pts, fill, egui::Stroke::NONE));
                }
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

            // Subtract nested room areas from outer room display
            let mut display_area = metrics.net_area;
            if let Some(nested_indices) = nested_in.get(&i) {
                for &nested_idx in nested_indices {
                    if let Some(nested_m) = &room_metrics[nested_idx] {
                        display_area -= nested_m.net_area;
                    }
                }
                display_area = display_area.max(0.0);
            }

            let area_m2 = display_area / 1_000_000.0;
            painter.text(
                egui::pos2(cx, cy + 16.0 * self.label_scale),
                egui::Align2::CENTER_CENTER,
                format!("{:.1} м²", area_m2),
                egui::FontId::proportional(11.0 * self.label_scale),
                label_color,
            );
        }
    }

    // -----------------------------------------------------------------------
    // draw_labels
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

    // -----------------------------------------------------------------------
    // draw_opening_preview — reuses door/window symbol functions
    // -----------------------------------------------------------------------

    pub(super) fn draw_opening_preview(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();
        let wall_id = match self.editor.opening_tool.hover_wall_id {
            Some(id) => id,
            None => return,
        };
        let wall = match self.project.wall(wall_id) {
            Some(w) => w,
            None => return,
        };

        let geo = match WallScreenGeometry::from_wall(wall, &self.editor.canvas, center) {
            Some(g) => g,
            None => return,
        };

        let opening_width = if self.editor.active_tool == EditorTool::Door {
            self.project.defaults.door_width
        } else {
            self.project.defaults.window_width
        };

        let offset = self.editor.opening_tool.hover_offset;
        let wall_len = wall.length();
        if wall_len < 1.0 {
            return;
        }

        let half_w = opening_width / 2.0;
        let t_center = (offset / wall_len) as f32;
        let t_left = ((offset - half_w) / wall_len).clamp(0.0, 1.0) as f32;
        let t_right = ((offset + half_w) / wall_len).clamp(0.0, 1.0) as f32;

        let p_left = geo.lerp(t_left);
        let p_right = geo.lerp(t_right);
        let p_center = geo.lerp(t_center);

        let preview_color = if self.editor.active_tool == EditorTool::Door {
            egui::Color32::from_rgba_premultiplied(180, 120, 60, 160)
        } else {
            egui::Color32::from_rgba_premultiplied(80, 160, 220, 160)
        };

        let corners = [
            egui::pos2(p_left.x + geo.nx, p_left.y + geo.ny),
            egui::pos2(p_right.x + geo.nx, p_right.y + geo.ny),
            egui::pos2(p_right.x - geo.nx, p_right.y - geo.ny),
            egui::pos2(p_left.x - geo.nx, p_left.y - geo.ny),
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
            egui::pos2(p_center.x, p_center.y - geo.half_thick - 10.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            egui::FontId::proportional(11.0 * self.label_scale),
            preview_color,
        );
    }
}

// ---------------------------------------------------------------------------
// Triangulation helper
// ---------------------------------------------------------------------------

/// Triangulate a simple polygon using earcutr (earcut algorithm).
/// Input: vertices in order (CCW or CW).
/// Output: list of triangle index triples [i, j, k] referencing input vertices.
fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]> {
    if vertices.len() < 3 {
        return Vec::new();
    }
    let coords: Vec<f32> = vertices.iter().flat_map(|p| [p.x, p.y]).collect();
    let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
    indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()
}
