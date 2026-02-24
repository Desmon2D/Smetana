use eframe::egui;
use glam::DVec2;
use uuid::Uuid;

use crate::model::Point;

// ---------------------------------------------------------------------------
// Canvas (viewport)
// ---------------------------------------------------------------------------

const MIN_ZOOM: f32 = 0.02;
const MAX_ZOOM: f32 = 5.0;

pub struct Canvas {
    /// Pan offset in world coordinates (mm).
    pub offset: egui::Vec2,
    /// Zoom level (pixels per mm).
    pub zoom: f32,
    /// Grid step size in mm.
    pub grid_step: f64,
    /// Current cursor position in world coordinates (mm), if hovering over canvas.
    pub cursor_world_pos: Option<DVec2>,
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
    /// Convert world coordinates (mm) to screen coordinates (pixels).
    pub fn world_to_screen(&self, world: egui::Pos2, rect_center: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            (world.x + self.offset.x) * self.zoom + rect_center.x,
            (world.y + self.offset.y) * self.zoom + rect_center.y,
        )
    }

    /// Convert screen coordinates (pixels) to world coordinates (mm).
    pub fn screen_to_world(&self, screen: egui::Pos2, rect_center: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            (screen.x - rect_center.x) / self.zoom - self.offset.x,
            (screen.y - rect_center.y) / self.zoom - self.offset.y,
        )
    }

    /// Convert screen coordinates to world coordinates as `DVec2` (mm).
    pub fn screen_to_world_dvec2(
        &self,
        screen: egui::Pos2,
        rect_center: egui::Pos2,
    ) -> DVec2 {
        let p = self.screen_to_world(screen, rect_center);
        DVec2::new(p.x as f64, p.y as f64)
    }

    /// Convert world coordinates (`DVec2` mm) to screen coordinates (pixels).
    pub fn dvec2_to_screen(&self, world: DVec2, rect_center: egui::Pos2) -> egui::Pos2 {
        self.world_to_screen(egui::pos2(world.x as f32, world.y as f32), rect_center)
    }

    /// Pan by a screen-space delta (pixels).
    pub(super) fn pan(&mut self, screen_delta: egui::Vec2) {
        self.offset += screen_delta / self.zoom;
    }

    /// Zoom toward a screen point, keeping the world point under the cursor stable.
    pub(super) fn zoom_toward(
        &mut self,
        screen_pos: egui::Pos2,
        rect_center: egui::Pos2,
        factor: f32,
    ) {
        let world_before = self.screen_to_world(screen_pos, rect_center);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.offset.x = (screen_pos.x - rect_center.x) / self.zoom - world_before.x;
        self.offset.y = (screen_pos.y - rect_center.y) / self.zoom - world_before.y;
    }

    /// Render the background grid on the canvas.
    pub(super) fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
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
// VisibilityMode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityMode {
    All,
    Wireframe,
    Rooms,
}

impl VisibilityMode {
    pub fn show_room_fills(&self) -> bool {
        matches!(self, Self::All | Self::Rooms)
    }

    pub fn show_wall_fills(&self) -> bool {
        matches!(self, Self::All)
    }

    pub fn show_opening_fills(&self) -> bool {
        matches!(self, Self::All)
    }
}

// ---------------------------------------------------------------------------
// Snap
// ---------------------------------------------------------------------------

const POINT_SNAP_RADIUS: f64 = 15.0;

pub struct SnapResult {
    pub position: DVec2,
    pub snapped_point: Option<Uuid>,
}

/// Find the nearest existing point within the screen-space snap radius.
pub fn snap_to_point(world_pos: DVec2, points: &[Point], zoom: f32) -> Option<Uuid> {
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

/// Snap a world position to the nearest grid intersection.
fn snap_to_grid(world_pos: DVec2, grid_step: f64) -> DVec2 {
    DVec2::new(
        (world_pos.x / grid_step).round() * grid_step,
        (world_pos.y / grid_step).round() * grid_step,
    )
}

/// Combined snap: try point snap first, then grid snap.
/// If `snap_enabled` is false, returns the raw world position.
pub fn snap(
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
