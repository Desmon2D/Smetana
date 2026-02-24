use eframe::egui;

const MIN_ZOOM: f32 = 0.02; // 0.02 px/mm → 1 m = 20 px
const MAX_ZOOM: f32 = 5.0; // 5 px/mm → 1 m = 5000 px

/// Viewport state for the 2D canvas.
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
            zoom: 0.5, // 0.5 px per mm → 1 m = 500 px
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

    /// Convert screen coordinates to world coordinates as a `DVec2` (mm).
    pub fn screen_to_world_dvec2(&self, screen: egui::Pos2, rect_center: egui::Pos2) -> glam::DVec2 {
        let p = self.screen_to_world(screen, rect_center);
        glam::DVec2::new(p.x as f64, p.y as f64)
    }

    /// Pan by a screen-space delta (pixels).
    pub fn pan(&mut self, screen_delta: egui::Vec2) {
        self.offset += screen_delta / self.zoom;
    }

    /// Zoom toward a screen point, keeping the world point under the cursor stable.
    pub fn zoom_toward(&mut self, screen_pos: egui::Pos2, rect_center: egui::Pos2, factor: f32) {
        let world_before = self.screen_to_world(screen_pos, rect_center);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // Adjust offset so world_before remains at screen_pos
        self.offset.x = (screen_pos.x - rect_center.x) / self.zoom - world_before.x;
        self.offset.y = (screen_pos.y - rect_center.y) / self.zoom - world_before.y;
    }

    /// Render the background grid on the canvas.
    pub fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let center = rect.center();

        // Minimum screen-space pixel spacing before we show a grid level
        let min_px = 20.0;

        let minor_step = self.grid_step as f32; // 100 mm
        let major_step = minor_step * 10.0; // 1000 mm = 1 m
        let sub_step = minor_step / 10.0; // 10 mm

        // Sub-minor grid (10 mm) — only at high zoom
        let sub_px = sub_step * self.zoom;
        if sub_px >= min_px {
            self.draw_grid_lines(
                painter,
                rect,
                center,
                sub_step,
                egui::Color32::from_rgb(55, 55, 60),
            );
        }

        // Minor grid (100 mm)
        let minor_px = minor_step * self.zoom;
        if minor_px >= min_px {
            self.draw_grid_lines(
                painter,
                rect,
                center,
                minor_step,
                egui::Color32::from_rgb(65, 65, 72),
            );
        }

        // Major grid (1 m)
        let major_px = major_step * self.zoom;
        if major_px >= min_px {
            self.draw_grid_lines(
                painter,
                rect,
                center,
                major_step,
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
        // Visible world-space bounds
        let tl = self.screen_to_world(rect.left_top(), center);
        let br = self.screen_to_world(rect.right_bottom(), center);

        let x0 = (tl.x / step).floor() as i64;
        let x1 = (br.x / step).ceil() as i64;
        let y0 = (tl.y / step).floor() as i64;
        let y1 = (br.y / step).ceil() as i64;

        let stroke = egui::Stroke::new(1.0, color);

        // Vertical lines
        for i in x0..=x1 {
            let sx = self
                .world_to_screen(egui::pos2(i as f32 * step, 0.0), center)
                .x;
            painter.line_segment(
                [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                stroke,
            );
        }

        // Horizontal lines
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
