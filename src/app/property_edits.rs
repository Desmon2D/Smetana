use eframe::egui;

use crate::editor::Selection;
use crate::history::{ModifyOpeningCommand, ModifyWallCommand, WallProps};
use crate::model::{Opening, OpeningKind, SideData, TargetObjectType};
use super::App;

fn opening_kind_changed(a: &OpeningKind, b: &OpeningKind) -> bool {
    match (a, b) {
        (
            OpeningKind::Door { height: h1, width: w1 },
            OpeningKind::Door { height: h2, width: w2 },
        ) => (h1 - h2).abs() > 0.01 || (w1 - w2).abs() > 0.01,
        (
            OpeningKind::Window { height: h1, width: w1, sill_height: s1, reveal_width: r1 },
            OpeningKind::Window { height: h2, width: w2, sill_height: s2, reveal_width: r2 },
        ) => {
            (h1 - h2).abs() > 0.01 || (w1 - w2).abs() > 0.01
                || (s1 - s2).abs() > 0.01 || (r1 - r2).abs() > 0.01
        }
        _ => true,
    }
}

impl App {
    pub(super) fn update_edit_snapshots(&mut self) {
        let wall_snap_matches = match (&self.wall_edit_snapshot, self.editor.selection) {
            (Some((snap_id, ..)), Selection::Wall(sel_id)) => *snap_id == sel_id,
            (None, _) => true,
            _ => false,
        };
        if !wall_snap_matches {
            self.flush_property_edits();
        }

        let opening_snap_matches = match (&self.opening_edit_snapshot, self.editor.selection) {
            (Some((snap_id, _)), Selection::Opening(sel_id)) => *snap_id == sel_id,
            (None, _) => true,
            _ => false,
        };
        if !opening_snap_matches {
            self.flush_property_edits();
        }
    }

    pub(super) fn flush_property_edits(&mut self) {
        if let Some((snap_id, old_props)) = self.wall_edit_snapshot.take() {
            if let Some(wall) = self.project.walls.iter().find(|w| w.id == snap_id) {
                let sections_changed = |old: &SideData, new: &SideData| -> bool {
                    if old.sections.len() != new.sections.len() {
                        return true;
                    }
                    old.sections.iter().zip(new.sections.iter()).any(|(a, b)| {
                        (a.length - b.length).abs() > 0.01
                            || (a.height_start - b.height_start).abs() > 0.01
                            || (a.height_end - b.height_end).abs() > 0.01
                    })
                };
                let changed = (wall.thickness - old_props.thickness).abs() > 0.01
                    || (wall.left_side.length - old_props.left_side.length).abs() > 0.01
                    || (wall.left_side.height_start - old_props.left_side.height_start).abs() > 0.01
                    || (wall.left_side.height_end - old_props.left_side.height_end).abs() > 0.01
                    || (wall.right_side.length - old_props.right_side.length).abs() > 0.01
                    || (wall.right_side.height_start - old_props.right_side.height_start).abs() > 0.01
                    || (wall.right_side.height_end - old_props.right_side.height_end).abs() > 0.01
                    || sections_changed(&old_props.left_side, &wall.left_side)
                    || sections_changed(&old_props.right_side, &wall.right_side);
                if changed {
                    let new_props = WallProps {
                        thickness: wall.thickness,
                        left_side: wall.left_side.clone(),
                        right_side: wall.right_side.clone(),
                    };
                    self.history.push_already_applied(Box::new(
                        ModifyWallCommand::new(snap_id, old_props, new_props),
                    ));
                }
            }
        }
        if let Some((snap_id, old_kind)) = self.opening_edit_snapshot.take() {
            if let Some(opening) = self.project.openings.iter().find(|o| o.id == snap_id) {
                if opening_kind_changed(&old_kind, &opening.kind) {
                    self.history.push_already_applied(Box::new(
                        ModifyOpeningCommand::from_values(snap_id, old_kind, opening.kind.clone()),
                    ));
                }
            }
        }
    }

    pub(super) fn has_validation_errors(&self) -> bool {
        for opening in &self.project.openings {
            if opening.wall_id.is_none() {
                return true;
            }
            if let Some(wid) = opening.wall_id {
                match self.project.walls.iter().find(|w| w.id == wid) {
                    None => return true,
                    Some(wall) => {
                        let wall_len = wall.length();
                        let half_w = opening.kind.width() / 2.0;
                        if opening.offset_along_wall - half_w < -1.0
                            || opening.offset_along_wall + half_w > wall_len + 1.0
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub(super) fn opening_errors(&self, opening: &Opening) -> Vec<&'static str> {
        let mut errors = Vec::new();
        match opening.wall_id {
            None => {
                errors.push("Проём не привязан к стене");
            }
            Some(wid) => match self.project.walls.iter().find(|w| w.id == wid) {
                None => {
                    errors.push("Стена не найдена");
                }
                Some(wall) => {
                    let wall_len = wall.length();
                    let half_w = opening.kind.width() / 2.0;
                    if opening.offset_along_wall - half_w < -1.0
                        || opening.offset_along_wall + half_w > wall_len + 1.0
                    {
                        errors.push("Проём выходит за пределы стены");
                    }
                }
            },
        }
        errors
    }

    pub(super) fn selection_target_type(&self) -> Option<TargetObjectType> {
        match self.editor.selection {
            Selection::Wall(_) => Some(TargetObjectType::Wall),
            Selection::Opening(id) => {
                self.project.openings.iter().find(|o| o.id == id).map(|o| match &o.kind {
                    OpeningKind::Door { .. } => TargetObjectType::Door,
                    OpeningKind::Window { .. } => TargetObjectType::Window,
                })
            }
            Selection::Room(_) => Some(TargetObjectType::Room),
            Selection::None => None,
        }
    }

    pub(super) fn show_side_sections(ui: &mut egui::Ui, side_data: &mut SideData, side_id: &str, section_net_areas: &[f64], color_offset: usize) {
        const SECTION_COLORS: &[(u8, u8, u8)] = &[
            (100, 180, 240),
            (240, 160, 100),
            (100, 220, 140),
            (220, 120, 220),
            (240, 220, 100),
            (120, 220, 220),
        ];

        ui.add_space(4.0);
        let mut changed = false;
        for i in 0..side_data.sections.len() {
            let global_idx = color_offset + i;
            let color_idx = global_idx % SECTION_COLORS.len();
            let (cr, cg, cb) = SECTION_COLORS[color_idx];
            let color = egui::Color32::from_rgb(cr, cg, cb);

            ui.horizontal(|ui| {
                ui.colored_label(color, "●");
                ui.label(format!("Секция {}", global_idx + 1));
            });
            ui.indent(format!("{side_id}_section_{i}"), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Длина (мм):");
                    if ui.add(
                        egui::DragValue::new(&mut side_data.sections[i].length)
                            .range(1.0..=100000.0)
                            .speed(10.0),
                    ).changed() {
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Выс. начала (мм):");
                    if ui.add(
                        egui::DragValue::new(&mut side_data.sections[i].height_start)
                            .range(100.0..=10000.0)
                            .speed(10.0),
                    ).changed() {
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Выс. конца (мм):");
                    if ui.add(
                        egui::DragValue::new(&mut side_data.sections[i].height_end)
                            .range(100.0..=10000.0)
                            .speed(10.0),
                    ).changed() {
                        changed = true;
                    }
                });
                let gross_m2 = side_data.sections[i].gross_area() / 1_000_000.0;
                ui.horizontal(|ui| {
                    ui.label("Площадь (брутто):");
                    ui.label(format!("{:.2} м²", gross_m2));
                });
                let net_m2 = section_net_areas.get(i).copied().unwrap_or(0.0) / 1_000_000.0;
                ui.horizontal(|ui| {
                    ui.label("Площадь (нетто):");
                    ui.label(format!("{:.2} м²", net_m2));
                });
            });
        }
        if changed {
            side_data.sync_from_sections();
        }
    }
}
