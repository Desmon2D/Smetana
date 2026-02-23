use eframe::egui;

use crate::editor::{EditorTool, Selection, SnapType, WallToolState, snap};
use crate::editor::room_detection::WallGraph;
use crate::history::{AddLabelCommand, AddOpeningCommand, AddWallCommand, RemoveLabelCommand, RemoveOpeningCommand, RemoveWallCommand};
use crate::model::{Label, Opening, OpeningKind, Point2D, Wall};
use super::App;

impl App {
    pub(super) fn show_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

            let rect = response.rect;

            if response.dragged_by(egui::PointerButton::Middle) {
                self.editor.canvas.pan(response.drag_delta());
            }

            let space_held = ui.input(|i| i.key_down(egui::Key::Space));
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

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(45, 45, 48));
            self.editor.canvas.draw_grid(&painter, rect);

            if let Some(pos) = response.hover_pos() {
                let world = self.editor.canvas.screen_to_world(pos, rect.center());
                self.editor.canvas.cursor_world_pos = Some(world);
            } else {
                self.editor.canvas.cursor_world_pos = None;
            }

            let shift_held = ui.input(|i| i.modifiers.shift);
            if self.editor.active_tool == EditorTool::Wall {
                if let Some(hover) = response.hover_pos() {
                    let world = self.editor.canvas.screen_to_world(hover, rect.center());
                    let world_pt = Point2D::new(world.x as f64, world.y as f64);
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
                                let junction_target = match &self.editor.wall_tool.last_snap {
                                    Some(snap_res) => match &snap_res.snap_type {
                                        SnapType::WallEdge { wall_id, side, t } => {
                                            Some((*wall_id, *side, *t))
                                        }
                                        _ => None,
                                    },
                                    None => None,
                                };

                                let closing = if let Some(chain_start) =
                                    self.editor.wall_tool.chain_start
                                {
                                    let snap_radius =
                                        15.0_f64 / self.editor.canvas.zoom as f64;
                                    snapped.distance_to(chain_start) < snap_radius
                                        && start.distance_to(chain_start) > 1.0
                                } else {
                                    false
                                };

                                if closing {
                                    let chain_start =
                                        self.editor.wall_tool.chain_start.unwrap();
                                    // The closing wall ends at chain_start which may
                                    // have been a T-junction (wall edge snap).
                                    let end_junction = match &self.editor.wall_tool.chain_start_snap {
                                        Some(snap_res) => match &snap_res.snap_type {
                                            SnapType::WallEdge { wall_id, side, t } => {
                                                Some((*wall_id, *side, *t))
                                            }
                                            _ => None,
                                        },
                                        None => None,
                                    };
                                    let wall = Wall::new(start, chain_start, self.project.defaults.wall_thickness, self.project.defaults.wall_height);
                                    self.flush_property_edits();
                                    self.history.push(
                                        Box::new(AddWallCommand {
                                            wall,
                                            junction_target: end_junction,
                                            start_junction_target: None,
                                        }),
                                        &mut self.project,
                                    );
                                    self.editor.wall_tool.reset();
                                } else if start.distance_to(snapped) > 1.0 {
                                    // Start-point junction: only for the first wall
                                    // of the chain (start == chain_start). Subsequent
                                    // chained walls inherit the previous wall's end
                                    // junction, so no double-registration.
                                    let is_first_in_chain = self.editor.wall_tool.chain_start
                                        .map_or(false, |cs| cs.distance_to(start) < 1.0);
                                    let start_junction = if is_first_in_chain {
                                        match &self.editor.wall_tool.start_snap {
                                            Some(snap_res) => match &snap_res.snap_type {
                                                SnapType::WallEdge { wall_id, side, t } => {
                                                    Some((*wall_id, *side, *t))
                                                }
                                                _ => None,
                                            },
                                            None => None,
                                        }
                                    } else {
                                        None
                                    };
                                    let wall = Wall::new(start, snapped, self.project.defaults.wall_thickness, self.project.defaults.wall_height);
                                    self.flush_property_edits();
                                    self.history.push(
                                        Box::new(AddWallCommand {
                                            wall,
                                            junction_target,
                                            start_junction_target: start_junction,
                                        }),
                                        &mut self.project,
                                    );
                                    // Save current snap as the start_snap for the next
                                    // chained wall (so it won't produce a junction).
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

            if self.editor.active_tool == EditorTool::Select {
                if response.clicked() && !space_held {
                    if let Some(hover) = response.hover_pos() {
                        let world = self.editor.canvas.screen_to_world(hover, rect.center());
                        let click_pt = Point2D::new(world.x as f64, world.y as f64);
                        let hit_tolerance = 10.0_f64 / self.editor.canvas.zoom as f64;

                        // Check labels first (small click targets)
                        let label_hit_tolerance = 20.0_f64 / self.editor.canvas.zoom as f64;
                        let mut best_label: Option<(uuid::Uuid, f64)> = None;
                        for label in &self.project.labels {
                            let dist = click_pt.distance_to(label.position);
                            if dist < label_hit_tolerance {
                                if best_label.is_none() || dist < best_label.unwrap().1 {
                                    best_label = Some((label.id, dist));
                                }
                            }
                        }

                        if let Some((id, _)) = best_label {
                            self.editor.selection = Selection::Label(id);
                        } else {
                        let mut best_opening: Option<(uuid::Uuid, f64)> = None;
                        for opening in &self.project.openings {
                            if let Some(wid) = opening.wall_id {
                                if let Some(wall) =
                                    self.project.walls.iter().find(|w| w.id == wid)
                                {
                                    let wall_len = wall.length();
                                    if wall_len < 1.0 {
                                        continue;
                                    }
                                    let t = (opening.offset_along_wall / wall_len)
                                        .clamp(0.0, 1.0);
                                    let cx =
                                        wall.start.x + (wall.end.x - wall.start.x) * t;
                                    let cy =
                                        wall.start.y + (wall.end.y - wall.start.y) * t;
                                    let dist =
                                        click_pt.distance_to(Point2D::new(cx, cy));
                                    let threshold =
                                        opening.kind.width() / 2.0 + hit_tolerance;
                                    if dist < threshold {
                                        if best_opening.is_none()
                                            || dist < best_opening.unwrap().1
                                        {
                                            best_opening = Some((opening.id, dist));
                                        }
                                    }
                                }
                            } else if let Some(pos) = opening.fallback_position {
                                let dist = click_pt.distance_to(pos);
                                let threshold = opening.kind.width() / 2.0 + hit_tolerance;
                                if dist < threshold {
                                    if best_opening.is_none()
                                        || dist < best_opening.unwrap().1
                                    {
                                        best_opening = Some((opening.id, dist));
                                    }
                                }
                            }
                        }

                        if let Some((id, _)) = best_opening {
                            self.editor.selection = Selection::Opening(id);
                        } else {
                            let mut best_wall: Option<(uuid::Uuid, f64)> = None;
                            for wall in &self.project.walls {
                                let dist =
                                    click_pt.distance_to_segment(wall.start, wall.end);
                                let threshold = wall.thickness / 2.0 + hit_tolerance;
                                if dist < threshold {
                                    if best_wall.is_none()
                                        || dist < best_wall.unwrap().1
                                    {
                                        best_wall = Some((wall.id, dist));
                                    }
                                }
                            }
                            self.editor.selection = match best_wall {
                                Some((id, _)) => Selection::Wall(id),
                                None => Selection::None,
                            };
                        }
                        } // else (label not hit)
                    }
                }

                if response.dragged_by(egui::PointerButton::Primary) && !space_held {
                    if let Selection::Label(lid) = self.editor.selection {
                        if let Some(hover) = response.hover_pos() {
                            let world =
                                self.editor.canvas.screen_to_world(hover, rect.center());
                            let cursor_pt =
                                Point2D::new(world.x as f64, world.y as f64);
                            if let Some(label) = self
                                .project
                                .labels
                                .iter_mut()
                                .find(|l| l.id == lid)
                            {
                                label.position = cursor_pt;
                            }
                        }
                    } else if let Selection::Opening(oid) = self.editor.selection {
                        if let Some(hover) = response.hover_pos() {
                            let world =
                                self.editor.canvas.screen_to_world(hover, rect.center());
                            let cursor_pt =
                                Point2D::new(world.x as f64, world.y as f64);
                            let hit_tolerance =
                                10.0_f64 / self.editor.canvas.zoom as f64;

                            let mut best: Option<(uuid::Uuid, f64, f64)> = None;
                            for wall in &self.project.walls {
                                let dist = cursor_pt
                                    .distance_to_segment(wall.start, wall.end);
                                let threshold = wall.thickness / 2.0 + hit_tolerance;
                                if dist < threshold {
                                    if best.is_none() || dist < best.unwrap().1 {
                                        let (t, _) = cursor_pt
                                            .project_onto_segment(wall.start, wall.end);
                                        let offset = t * wall.length();
                                        best = Some((wall.id, dist, offset));
                                    }
                                }
                            }

                            let old_wall_id = self
                                .project
                                .openings
                                .iter()
                                .find(|o| o.id == oid)
                                .and_then(|o| o.wall_id);

                            if let Some((new_wall_id, _, new_offset)) = best {
                                if let Some(opening) = self
                                    .project
                                    .openings
                                    .iter_mut()
                                    .find(|o| o.id == oid)
                                {
                                    opening.wall_id = Some(new_wall_id);
                                    opening.offset_along_wall = new_offset;
                                }
                                if old_wall_id != Some(new_wall_id) {
                                    if let Some(prev_wid) = old_wall_id {
                                        if let Some(w) = self
                                            .project
                                            .walls
                                            .iter_mut()
                                            .find(|w| w.id == prev_wid)
                                        {
                                            w.openings.retain(|id| *id != oid);
                                        }
                                    }
                                    if let Some(w) = self
                                        .project
                                        .walls
                                        .iter_mut()
                                        .find(|w| w.id == new_wall_id)
                                    {
                                        if !w.openings.contains(&oid) {
                                            w.openings.push(oid);
                                        }
                                    }
                                }
                            } else {
                                let fb_pos = self.project.openings.iter()
                                    .find(|o| o.id == oid)
                                    .and_then(|opening| {
                                        let wid = opening.wall_id?;
                                        let wall = self.project.walls.iter().find(|w| w.id == wid)?;
                                        let wall_len = wall.length();
                                        if wall_len > 0.0 {
                                            let t = opening.offset_along_wall / wall_len;
                                            Some(Point2D::new(
                                                wall.start.x + (wall.end.x - wall.start.x) * t,
                                                wall.start.y + (wall.end.y - wall.start.y) * t,
                                            ))
                                        } else {
                                            None
                                        }
                                    });
                                if let Some(opening) = self
                                    .project
                                    .openings
                                    .iter_mut()
                                    .find(|o| o.id == oid)
                                {
                                    opening.fallback_position = fb_pos;
                                    opening.wall_id = None;
                                }
                                if let Some(prev_wid) = old_wall_id {
                                    if let Some(w) = self
                                        .project
                                        .walls
                                        .iter_mut()
                                        .find(|w| w.id == prev_wid)
                                    {
                                        w.openings.retain(|id| *id != oid);
                                    }
                                }
                            }
                        }
                    }
                }

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.editor.selection = Selection::None;
                }

                if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                    match self.editor.selection {
                        Selection::Wall(id) => {
                            self.flush_property_edits();
                            if let Some(cmd) = RemoveWallCommand::new(id, &self.project) {
                                self.history
                                    .push(Box::new(cmd), &mut self.project);
                            }
                            self.editor.selection = Selection::None;
                        }
                        Selection::Opening(id) => {
                            self.flush_property_edits();
                            if let Some(cmd) = RemoveOpeningCommand::new(id, &self.project) {
                                self.history
                                    .push(Box::new(cmd), &mut self.project);
                            }
                            self.editor.selection = Selection::None;
                        }
                        Selection::Label(id) => {
                            self.flush_property_edits();
                            if let Some(cmd) = RemoveLabelCommand::new(id, &self.project) {
                                self.history
                                    .push(Box::new(cmd), &mut self.project);
                            }
                            self.editor.selection = Selection::None;
                        }
                        _ => {}
                    }
                }
            }

            if self.editor.active_tool == EditorTool::Door
                || self.editor.active_tool == EditorTool::Window
            {
                self.editor.opening_tool.hover_wall_id = None;
                if let Some(hover) = response.hover_pos() {
                    let world = self.editor.canvas.screen_to_world(hover, rect.center());
                    let cursor_pt = Point2D::new(world.x as f64, world.y as f64);
                    let hit_tolerance = 10.0_f64 / self.editor.canvas.zoom as f64;

                    let mut best: Option<(uuid::Uuid, f64, f64)> = None;
                    for wall in &self.project.walls {
                        let dist = cursor_pt.distance_to_segment(wall.start, wall.end);
                        let threshold = wall.thickness / 2.0 + hit_tolerance;
                        if dist < threshold {
                            if best.is_none() || dist < best.unwrap().1 {
                                let (t, _proj) =
                                    cursor_pt.project_onto_segment(wall.start, wall.end);
                                let offset = t * wall.length();
                                best = Some((wall.id, dist, offset));
                            }
                        }
                    }

                    if let Some((wall_id, _dist, offset)) = best {
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
                        self.flush_property_edits();
                        self.history.push(
                            Box::new(AddOpeningCommand { opening }),
                            &mut self.project,
                        );
                        self.editor.selection = Selection::Opening(opening_id);
                    }
                }
            }

            if self.editor.active_tool == EditorTool::Label {
                if response.clicked() && !space_held {
                    if let Some(hover) = response.hover_pos() {
                        let world = self.editor.canvas.screen_to_world(hover, rect.center());
                        let world_pt = Point2D::new(world.x as f64, world.y as f64);
                        let label = Label::new("Подпись".to_string(), world_pt);
                        let label_id = label.id;
                        self.flush_property_edits();
                        self.history.push(
                            Box::new(AddLabelCommand { label }),
                            &mut self.project,
                        );
                        self.editor.selection = Selection::Label(label_id);
                    }
                }
            }

            let graph = WallGraph::build(&self.project.walls);
            let new_rooms = graph.detect_rooms(&self.project.walls);
            self.merge_rooms(new_rooms);

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
        });
    }
}
