use eframe::egui;

use crate::editor::Selection;
use crate::model::room_metrics::compute_room_metrics;
use crate::model::{Opening, OpeningKind, SideData, TargetObjectType, Wall, WallSide, section_net_area};
use super::{App, ServiceTarget, property_edits};

struct SideInfo {
    has_junctions: bool,
    total_length: f64,
    avg_height: f64,
    gross_m2: f64,
    net_m2: f64,
    section_net_areas: Vec<f64>,
}

impl SideInfo {
    fn compute(wall: &Wall, side: WallSide, openings: &[Opening], walls: &[Wall]) -> Self {
        let side_data = match side {
            WallSide::Left => &wall.left_side,
            WallSide::Right => &wall.right_side,
        };
        let section_nets: Vec<f64> = (0..side_data.sections.len())
            .map(|i| section_net_area(wall, side, i, openings))
            .collect();
        let gross = side_data.computed_gross_area();
        let open_area = crate::model::opening_area_mm2(wall, openings);
        SideInfo {
            has_junctions: !side_data.junctions.is_empty(),
            total_length: side_data.computed_total_length(walls),
            avg_height: side_data.average_height(),
            gross_m2: gross / 1_000_000.0,
            net_m2: (gross - open_area).max(0.0) / 1_000_000.0,
            section_net_areas: section_nets,
        }
    }

    fn empty() -> Self {
        SideInfo {
            has_junctions: false,
            total_length: 0.0,
            avg_height: 0.0,
            gross_m2: 0.0,
            net_m2: 0.0,
            section_net_areas: Vec::new(),
        }
    }
}

fn show_side_panel(
    ui: &mut egui::Ui,
    side_data: &mut SideData,
    info: &SideInfo,
    side_label: &str,
    side_id: &str,
    color_offset: usize,
) {
    ui.label(side_label);
    ui.indent(format!("{side_id}_side"), |ui| {
        let len = if info.has_junctions { info.total_length } else { side_data.length };
        property_edits::labeled_value(ui, "Длина (мм):", format!("{:.0}", len));
        property_edits::labeled_value(ui, "Средняя высота (мм):", format!("{:.0}", info.avg_height));
        property_edits::labeled_value(ui, "Площадь (брутто):", format!("{:.2} м²", info.gross_m2));
        property_edits::labeled_value(ui, "Площадь (нетто):", format!("{:.2} м²", info.net_m2));
        App::show_side_sections(ui, side_data, side_id, &info.section_net_areas, color_offset);
    });
}

impl App {
    pub(super) fn show_right_panel(&mut self, ctx: &egui::Context) {
        // Take a snapshot when editing starts so DragValue changes accumulate into one undo step
        if self.editor.selection != Selection::None
            && self.edit_snapshot_version != Some(self.history.version)
        {
            self.history.snapshot(&self.project, "edit properties");
            self.edit_snapshot_version = Some(self.history.version);
        }

        egui::SidePanel::right("right_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Свойства");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.editor.selection {
                        Selection::None => {
                            ui.label("Ничего не выбрано");
                        }
                        Selection::Wall(id) => self.show_wall_properties(ui, id),
                        Selection::Opening(id) => self.show_opening_properties(ui, id),
                        Selection::Room(id) => self.show_room_properties(ui, id),
                        Selection::Label(id) => self.show_label_properties(ui, id),
                    }
                }); // ScrollArea
            });
    }

    fn show_wall_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let (left_info, right_info) =
            if let Some(w) = self.project.wall(id) {
                (
                    SideInfo::compute(w, WallSide::Left, &self.project.openings, &self.project.walls),
                    SideInfo::compute(w, WallSide::Right, &self.project.openings, &self.project.walls),
                )
            } else {
                (SideInfo::empty(), SideInfo::empty())
            };

        if let Some(wall) = self.project.wall_mut(id) {
            ui.label("Стена");
            ui.add_space(8.0);

            property_edits::labeled_drag(ui, "Толщина (мм):", &mut wall.thickness, 10.0..=1000.0, 5.0);

            let length_mm = wall.length();
            let length_label = if length_mm >= 1000.0 {
                format!("{:.2} м ({:.0} мм)", length_mm / 1000.0, length_mm)
            } else {
                format!("{:.0} мм", length_mm)
            };
            property_edits::labeled_value(ui, "Длина (графика):", length_label);

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(60, 200, 80), "●");
                ui.label("Начало");
                ui.add_space(12.0);
                ui.colored_label(egui::Color32::from_rgb(230, 210, 50), "●");
                ui.label("Конец");
            });

            ui.add_space(8.0);

            let left_section_count = wall.left_side.sections.len();

            show_side_panel(ui, &mut wall.left_side, &left_info, "Левая сторона", "left", 0);
            ui.add_space(4.0);
            show_side_panel(ui, &mut wall.right_side, &right_info, "Правая сторона", "right", left_section_count);

            ui.add_space(8.0);
            ui.separator();
            ui.strong("Услуги");
            ui.add_space(4.0);
        } else {
            ui.label("Стена не найдена");
            self.editor.selection = Selection::None;
        }

        if let Some(wall) = self.project.wall(id) {
            let left_section_count = wall.left_side.sections.len();
            self.show_wall_side_services(ui, id, WallSide::Left, "Левая сторона", 0);
            ui.add_space(4.0);
            self.show_wall_side_services(ui, id, WallSide::Right, "Правая сторона", left_section_count);
        }
    }

    fn show_opening_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let errors: Vec<&str> = self
            .project
            .opening(id)
            .map(|o| self.opening_errors(o))
            .unwrap_or_default();

        let wall_thickness: Option<f64> = self
            .project
            .opening(id)
            .and_then(|o| o.wall_id)
            .and_then(|wid| self.project.wall(wid))
            .map(|w| w.thickness);

        if let Some(opening) =
            self.project.opening_mut(id)
        {
            let label = match &opening.kind {
                OpeningKind::Door { .. } => "Дверь",
                OpeningKind::Window { .. } => "Окно",
            };
            ui.label(label);
            ui.add_space(8.0);

            if !errors.is_empty() {
                for err in &errors {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 60, 60),
                        format!("⚠ {err}"),
                    );
                }
                ui.add_space(4.0);
            }

            match &mut opening.kind {
                OpeningKind::Door { height, width } => {
                    property_edits::labeled_drag(ui, "Высота (мм):", height, 500.0..=3500.0, 10.0);
                    property_edits::labeled_drag(ui, "Ширина (мм):", width, 300.0..=3000.0, 10.0);
                    if let Some(thick) = wall_thickness {
                        property_edits::labeled_value(ui, "Глубина (мм):", format!("{:.0}", thick));
                    }
                }
                OpeningKind::Window {
                    height,
                    width,
                    sill_height,
                    reveal_width,
                } => {
                    property_edits::labeled_drag(ui, "Высота (мм):", height, 200.0..=3000.0, 10.0);
                    property_edits::labeled_drag(ui, "Ширина (мм):", width, 200.0..=5000.0, 10.0);
                    property_edits::labeled_drag(ui, "Подоконник (мм):", sill_height, 0.0..=2500.0, 10.0);
                    property_edits::labeled_drag(ui, "Откос (мм):", reveal_width, 0.0..=500.0, 5.0);
                }
            }
        } else {
            ui.label("Проём не найден");
            self.editor.selection = Selection::None;
        }

        if let Some(opening) = self.project.opening(id) {
            ui.add_space(8.0);
            ui.separator();
            ui.strong("Услуги");
            ui.add_space(4.0);

            let svcs = self.project.opening_services.get(&id)
                .map(|v| v.as_slice()).unwrap_or(&[]);
            let rows = self.build_assigned_rows_for(svcs, |ut| {
                crate::model::opening_quantity(ut, opening)
            });
            let target = ServiceTarget::Opening { opening_id: id };
            self.show_flat_services(
                ui, id, target,
                self.selection_target_type().unwrap_or(TargetObjectType::Door),
                rows,
                |p| &mut p.opening_services,
            );
        }
    }

    fn show_room_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let metrics = self
            .project
            .room(id)
            .and_then(|r| compute_room_metrics(r, &self.project.walls));

        if let Some(room) =
            self.project.rooms.iter_mut().find(|r| r.id == id)
        {
            ui.label("Комната");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Название:");
                if ui.text_edit_singleline(&mut room.name).changed() {
                    self.history.mark_dirty();
                }
            });

            ui.add_space(4.0);

            if let Some(m) = &metrics {
                property_edits::labeled_value(ui, "Площадь (брутто):", format!("{:.2} м²", m.gross_area / 1_000_000.0));
                property_edits::labeled_value(ui, "Площадь (нетто):", format!("{:.2} м²", m.net_area / 1_000_000.0));
                property_edits::labeled_value(ui, "Периметр:", format!("{:.2} м", m.perimeter / 1000.0));
            }

            ui.add_space(4.0);
            property_edits::labeled_value(ui, "Стен в контуре:", format!("{}", room.wall_ids.len()));
        } else {
            ui.label("Комната не найдена");
            self.editor.selection = Selection::None;
        }

        if let Some(room) = self.project.room(id) {
            ui.add_space(8.0);
            ui.separator();
            ui.strong("Услуги");
            ui.add_space(4.0);

            let svcs = self.project.room_services.get(&id)
                .map(|v| v.as_slice()).unwrap_or(&[]);
            let walls = &self.project.walls;
            let rows = self.build_assigned_rows_for(svcs, |ut| {
                crate::model::room_quantity(ut, room, walls)
            });
            let target = ServiceTarget::Room { room_id: id };
            self.show_flat_services(
                ui, id, target,
                TargetObjectType::Room,
                rows,
                |p| &mut p.room_services,
            );
        }
    }

    fn show_label_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        // Auto-delete label if text was emptied
        let is_empty = self.project.labels.iter()
            .find(|l| l.id == id)
            .is_some_and(|l| l.text.trim().is_empty());
        if is_empty {
            self.project.labels.retain(|l| l.id != id);
            self.editor.selection = Selection::None;
            return;
        }

        if let Some(label) =
            self.project.labels.iter_mut().find(|l| l.id == id)
        {
            ui.label("Подпись");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Текст:");
                if ui.text_edit_singleline(&mut label.text).changed() {
                    self.history.mark_dirty();
                }
            });

            property_edits::labeled_drag(ui, "Размер шрифта:", &mut label.font_size, 6.0..=72.0, 0.5);

            let mut rotation_deg = label.rotation.to_degrees();
            if property_edits::labeled_drag(ui, "Поворот (°):", &mut rotation_deg, 0.0..=360.0, 1.0) {
                label.rotation = rotation_deg.to_radians();
            }
        } else {
            ui.label("Подпись не найдена");
            self.editor.selection = Selection::None;
        }
    }
}
