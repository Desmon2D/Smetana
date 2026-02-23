use eframe::egui;

use crate::editor::Selection;
use crate::model::{Opening, OpeningKind, SideData, TargetObjectType};
use super::{App, SECTION_COLORS};

impl App {
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
            Selection::Label(_) => None,
            Selection::None => None,
        }
    }

    pub(super) fn show_side_sections(ui: &mut egui::Ui, side_data: &mut SideData, side_id: &str, section_net_areas: &[f64], color_offset: usize) {
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
