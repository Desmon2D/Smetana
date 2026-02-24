use eframe::egui;

use crate::editor::{EditorTool, Selection, WallToolState, snap};
use crate::editor::room_detection::WallGraph;
use glam::DVec2;
use crate::model::{Label, Opening, OpeningKind, Wall, distance_to_segment, project_onto_segment};
use super::App;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Hit-test free functions (Step 15)
// ---------------------------------------------------------------------------

/// Find the nearest wall to `cursor` within `hit_tolerance`.
/// Returns `(wall_id, min_dist, t_param, offset_along_wall)`.
fn find_nearest_wall(
    cursor: DVec2,
    walls: &[Wall],
    hit_tolerance: f64,
) -> Option<(Uuid, f64, f64, f64)> {
    let mut best: Option<(Uuid, f64, f64, f64)> = None;
    for wall in walls {
        let dist = distance_to_segment(cursor, wall.start, wall.end);
        let threshold = wall.thickness / 2.0 + hit_tolerance;
        if dist < threshold {
            if best.is_none() || dist < best.unwrap().1 {
                let (t, _) = project_onto_segment(cursor, wall.start, wall.end);
                let offset = t * wall.length();
                best = Some((wall.id, dist, t, offset));
            }
        }
    }
    best
}

/// Find the nearest opening to `cursor` within `hit_tolerance`.
fn find_nearest_opening(
    cursor: DVec2,
    openings: &[Opening],
    walls: &[Wall],
    orphan_positions: &HashMap<Uuid, DVec2>,
    hit_tolerance: f64,
) -> Option<Uuid> {
    let mut best: Option<(Uuid, f64)> = None;
    for opening in openings {
        if let Some(wid) = opening.wall_id {
            if let Some(wall) = walls.iter().find(|w| w.id == wid) {
                let wall_len = wall.length();
                if wall_len < 1.0 {
                    continue;
                }
                let t = (opening.offset_along_wall / wall_len).clamp(0.0, 1.0);
                let center = wall.start + (wall.end - wall.start) * t;
                let dist = cursor.distance(center);
                let threshold = opening.kind.width() / 2.0 + hit_tolerance;
                if dist < threshold {
                    if best.is_none() || dist < best.unwrap().1 {
                        best = Some((opening.id, dist));
                    }
                }
            }
        } else if let Some(&pos) = orphan_positions.get(&opening.id) {
            let dist = cursor.distance(pos);
            let threshold = opening.kind.width() / 2.0 + hit_tolerance;
            if dist < threshold {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((opening.id, dist));
                }
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Find the nearest label to `cursor` within `hit_tolerance`.
fn find_nearest_label(
    cursor: DVec2,
    labels: &[Label],
    hit_tolerance: f64,
) -> Option<Uuid> {
    let mut best: Option<(Uuid, f64)> = None;
    for label in labels {
        let dist = cursor.distance(label.position);
        if dist < hit_tolerance {
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((label.id, dist));
            }
        }
    }
    best.map(|(id, _)| id)
}

// ---------------------------------------------------------------------------
// Orphan position helper (used by opening drag when detaching from wall)
// ---------------------------------------------------------------------------

/// Compute the world position of an opening from its wall attachment.
fn opening_world_position(opening: &Opening, walls: &[Wall]) -> Option<DVec2> {
    let wid = opening.wall_id?;
    let wall = walls.iter().find(|w| w.id == wid)?;
    let wall_len = wall.length();
    if wall_len > 0.0 {
        let t = opening.offset_along_wall / wall_len;
        Some(wall.start + (wall.end - wall.start) * t)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// App methods
// ---------------------------------------------------------------------------

impl App {
    // ---- Step 17: show_canvas orchestrator ------------------------------------

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
                EditorTool::Wall => self.handle_wall_tool(ui, &response, rect, shift_held, space_held),
                EditorTool::Select => self.handle_select_tool(ui, &response, rect, space_held),
                EditorTool::Door | EditorTool::Window => self.handle_opening_tool(&response, rect, space_held),
                EditorTool::Label => self.handle_label_tool(&response, rect, space_held),
            }

            if self.history.version != self.rooms_version {
                let graph = WallGraph::build(&self.project.walls);
                let new_rooms = graph.detect_rooms(&self.project.walls);
                self.merge_rooms(new_rooms);
                self.rooms_version = self.history.version;
            }

            self.draw_rooms(&painter, rect);
            self.draw_labels(&painter, rect);
            self.draw_walls(&painter, rect);
            self.draw_openings(&painter, rect);

            if self.editor.active_tool == EditorTool::Wall {
                self.draw_wall_preview(&painter, rect);
            }

            if (self.editor.active_tool == EditorTool::Door
                || self.editor.active_tool == EditorTool::Window)
                && self.editor.opening_tool.hover_wall_id.is_some()
            {
                self.draw_opening_preview(&painter, rect);
            }

            self.draw_empty_hint(&painter, rect);
            self.draw_status_bar(&painter, rect);
        });
    }

    // ---- Step 17: pan/zoom helper --------------------------------------------

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
                self.editor.canvas.zoom_toward(cursor, rect.center(), factor);
            }
        }
    }

    // ---- Step 17: status bar helper ------------------------------------------

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

    // ---- Step 17: empty-canvas hint helper -----------------------------------

    fn draw_empty_hint(&self, painter: &egui::Painter, rect: egui::Rect) {
        if self.project.walls.is_empty() {
            let tool_hint = match self.editor.active_tool {
                EditorTool::Select => "Режим выбора — кликните на объект",
                EditorTool::Wall => match self.editor.wall_tool.state {
                    WallToolState::Idle => "Кликните для начальной точки стены",
                    WallToolState::Drawing { .. } => "Кликните для конечной точки стены",
                },
                EditorTool::Door => "Режим двери — перетащите на стену",
                EditorTool::Window => "Режим окна — перетащите на стену",
                EditorTool::Label => "Режим надписи — кликните на холст",
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

    // ---- Wall tool (unchanged) -----------------------------------------------

    fn handle_wall_tool(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        shift_held: bool,
        space_held: bool,
    ) {
        if let Some(hover) = response.hover_pos() {
            let world_pt = self.editor.canvas.screen_to_world_dvec2(hover, rect.center());
            let snap_result = snap(
                world_pt,
                self.editor.canvas.grid_step,
                self.editor.canvas.zoom,
                &self.project.walls,
                shift_held,
            );
            self.editor.wall_tool.preview_end = Some(snap_result.position);
            self.editor.wall_tool.last_snap = Some(snap_result);
        } else {
            self.editor.wall_tool.preview_end = None;
            self.editor.wall_tool.last_snap = None;
        }

        if response.double_clicked() && !space_held {
            self.editor.wall_tool.reset();
        } else if response.clicked() && !space_held {
            if let Some(snapped) = self.editor.wall_tool.preview_end {
                match self.editor.wall_tool.state.clone() {
                    WallToolState::Idle => {
                        self.editor.wall_tool.chain_start = Some(snapped);
                        self.editor.wall_tool.start_snap =
                            self.editor.wall_tool.last_snap.clone();
                        self.editor.wall_tool.chain_start_snap =
                            self.editor.wall_tool.last_snap.clone();
                        self.editor.wall_tool.state =
                            WallToolState::Drawing { start: snapped };
                    }
                    WallToolState::Drawing { start } => {
                        let junction_target = self.editor.wall_tool.last_snap.as_ref().and_then(|s| s.wall_edge_junction());

                        let closing = if let Some(chain_start) =
                            self.editor.wall_tool.chain_start
                        {
                            let snap_radius =
                                15.0_f64 / self.editor.canvas.zoom as f64;
                            snapped.distance(chain_start) < snap_radius
                                && start.distance(chain_start) > 1.0
                        } else {
                            false
                        };

                        if closing {
                            let chain_start =
                                self.editor.wall_tool.chain_start.unwrap();
                            let end_junction = self.editor.wall_tool.chain_start_snap.as_ref().and_then(|s| s.wall_edge_junction());
                            let wall = Wall::new(start, chain_start, self.project.defaults.wall_thickness, self.project.defaults.wall_height);
                            self.history.snapshot(&self.project, "add wall");
                            self.project.add_wall(wall, end_junction, None);
                            self.editor.wall_tool.reset();
                        } else if start.distance(snapped) > 1.0 {
                            let is_first_in_chain = self.editor.wall_tool.chain_start
                                .map_or(false, |cs| cs.distance(start) < 1.0);
                            let start_junction = if is_first_in_chain {
                                self.editor.wall_tool.start_snap.as_ref().and_then(|s| s.wall_edge_junction())
                            } else {
                                None
                            };
                            let wall = Wall::new(start, snapped, self.project.defaults.wall_thickness, self.project.defaults.wall_height);
                            self.history.snapshot(&self.project, "add wall");
                            self.project.add_wall(wall, junction_target, start_junction);
                            self.editor.wall_tool.start_snap =
                                self.editor.wall_tool.last_snap.clone();
                            self.editor.wall_tool.chain_from(snapped);
                        }
                    }
                }
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor.wall_tool.reset();
        }
    }

    // ---- Step 16: select tool thin dispatcher --------------------------------

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

    // ---- Step 16: select click (cascading priority: labels > openings > walls)

    fn handle_select_click(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if !(response.clicked() && !space_held) {
            return;
        }
        self.edit_snapshot_version = None;

        let hover = match response.hover_pos() {
            Some(h) => h,
            None => return,
        };

        let click_pt = self.editor.canvas.screen_to_world_dvec2(hover, rect.center());
        let hit_tolerance = 10.0_f64 / self.editor.canvas.zoom as f64;
        let label_hit_tolerance = 20.0_f64 / self.editor.canvas.zoom as f64;

        // Priority 1: labels
        if let Some(id) = find_nearest_label(click_pt, &self.project.labels, label_hit_tolerance) {
            self.editor.selection = Selection::Label(id);
            return;
        }

        // Priority 2: openings
        if let Some(id) = find_nearest_opening(
            click_pt,
            &self.project.openings,
            &self.project.walls,
            &self.editor.orphan_positions,
            hit_tolerance,
        ) {
            self.editor.selection = Selection::Opening(id);
            return;
        }

        // Priority 3: walls
        self.editor.selection = match find_nearest_wall(click_pt, &self.project.walls, hit_tolerance) {
            Some((id, _, _, _)) => Selection::Wall(id),
            None => Selection::None,
        };
    }

    // ---- Step 16: select drag (label drag + opening drag via move_opening) ---

    fn handle_select_drag(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if response.drag_started() && !space_held {
            if matches!(self.editor.selection, Selection::Label(_) | Selection::Opening(_)) {
                self.history.snapshot(&self.project, "drag");
            }
        }

        if !(response.dragged_by(egui::PointerButton::Primary) && !space_held) {
            return;
        }

        let hover = match response.hover_pos() {
            Some(h) => h,
            None => return,
        };
        let cursor_pt = self.editor.canvas.screen_to_world_dvec2(hover, rect.center());

        if let Selection::Label(lid) = self.editor.selection {
            if let Some(label) = self.project.labels.iter_mut().find(|l| l.id == lid) {
                label.position = cursor_pt;
            }
        } else if let Selection::Opening(oid) = self.editor.selection {
            let hit_tolerance = 10.0_f64 / self.editor.canvas.zoom as f64;

            if let Some((new_wall_id, _, _, new_offset)) =
                find_nearest_wall(cursor_pt, &self.project.walls, hit_tolerance)
            {
                // Snap to a wall
                self.project.move_opening(oid, Some(new_wall_id), new_offset);
            } else {
                // Detach from wall: compute orphan position before detaching
                let fb_pos = self.project.opening(oid)
                    .and_then(|o| opening_world_position(o, &self.project.walls));
                if let Some(pos) = fb_pos {
                    self.editor.orphan_positions.insert(oid, pos);
                }
                self.project.move_opening(oid, None, 0.0);
            }
        }
    }

    // ---- Step 16: select keys (Escape and Delete) ----------------------------

    fn handle_select_keys(&mut self, ui: &egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor.selection = Selection::None;
        }

        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            self.delete_selected();
        }
    }

    // ---- Opening tool --------------------------------------------------------

    fn handle_opening_tool(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        self.editor.opening_tool.hover_wall_id = None;
        if let Some(hover) = response.hover_pos() {
            let cursor_pt = self.editor.canvas.screen_to_world_dvec2(hover, rect.center());
            let hit_tolerance = 10.0_f64 / self.editor.canvas.zoom as f64;

            if let Some((wall_id, _, _, offset)) =
                find_nearest_wall(cursor_pt, &self.project.walls, hit_tolerance)
            {
                self.editor.opening_tool.hover_wall_id = Some(wall_id);
                self.editor.opening_tool.hover_offset = offset;
            }
        }

        if response.clicked() && !space_held {
            if let Some(wall_id) = self.editor.opening_tool.hover_wall_id {
                let offset = self.editor.opening_tool.hover_offset;
                let kind = if self.editor.active_tool == EditorTool::Door {
                    OpeningKind::Door {
                        height: self.project.defaults.door_height,
                        width: self.project.defaults.door_width,
                    }
                } else {
                    OpeningKind::Window {
                        height: self.project.defaults.window_height,
                        width: self.project.defaults.window_width,
                        sill_height: self.project.defaults.window_sill_height,
                        reveal_width: self.project.defaults.window_reveal_width,
                    }
                };
                let opening = Opening::new(kind, Some(wall_id), offset);
                let opening_id = opening.id;
                self.history.snapshot(&self.project, "add opening");
                self.project.add_opening(opening);
                self.editor.selection = Selection::Opening(opening_id);
            }
        }
    }

    // ---- Label tool ----------------------------------------------------------

    fn handle_label_tool(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        space_held: bool,
    ) {
        if response.clicked() && !space_held {
            if let Some(hover) = response.hover_pos() {
                let world_pt = self.editor.canvas.screen_to_world_dvec2(hover, rect.center());
                let label = Label::new("Подпись".to_string(), world_pt);
                let label_id = label.id;
                self.history.snapshot(&self.project, "add label");
                self.project.labels.push(label);
                self.editor.selection = Selection::Label(label_id);
            }
        }
    }
}
