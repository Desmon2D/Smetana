use eframe::egui;

use crate::editor::Selection;
use crate::editor::room_detection::compute_room_metrics;
use crate::history::{ModifyOpeningCommand, ModifyWallCommand, WallProps};
use crate::model::{Opening, OpeningKind, SideData, TargetObjectType, WallSide};
use super::{App, ServiceTarget};

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
                let changed = (wall.thickness - old_props.thickness).abs() > 0.01
                    || (wall.left_side.length - old_props.left_side.length).abs() > 0.01
                    || (wall.left_side.height_start - old_props.left_side.height_start).abs() > 0.01
                    || (wall.left_side.height_end - old_props.left_side.height_end).abs() > 0.01
                    || (wall.right_side.length - old_props.right_side.length).abs() > 0.01
                    || (wall.right_side.height_start - old_props.right_side.height_start).abs() > 0.01
                    || (wall.right_side.height_end - old_props.right_side.height_end).abs() > 0.01;
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

    pub(super) fn show_side_sections(ui: &mut egui::Ui, side_data: &SideData, side_id: &str) {
        if !side_data.has_sections() {
            return;
        }

        const SECTION_COLORS: &[(u8, u8, u8)] = &[
            (100, 180, 240),
            (240, 160, 100),
            (100, 220, 140),
            (220, 120, 220),
            (240, 220, 100),
            (120, 220, 220),
        ];

        ui.add_space(4.0);
        for (i, section) in side_data.sections.iter().enumerate() {
            let color_idx = i % SECTION_COLORS.len();
            let (cr, cg, cb) = SECTION_COLORS[color_idx];
            let color = egui::Color32::from_rgb(cr, cg, cb);

            ui.horizontal(|ui| {
                ui.colored_label(color, "●");
                ui.label(format!("Секция {}", i + 1));
            });
            ui.indent(format!("{side_id}_section_{i}"), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Длина:");
                    ui.label(format!("{:.0} мм", section.length));
                });
                ui.horizontal(|ui| {
                    ui.label("Выс. начала:");
                    ui.label(format!("{:.0} мм", section.height_start));
                });
                ui.horizontal(|ui| {
                    ui.label("Выс. конца:");
                    ui.label(format!("{:.0} мм", section.height_end));
                });
                let area_m2 = section.gross_area() / 1_000_000.0;
                ui.horizontal(|ui| {
                    ui.label("Площадь:");
                    ui.label(format!("{:.2} м²", area_m2));
                });
            });
        }
    }

    pub(super) fn show_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Свойства");
                ui.separator();

                match self.editor.selection {
                    Selection::None => {
                        ui.label("Ничего не выбрано");
                    }
                    Selection::Wall(id) => {
                        if self.wall_edit_snapshot.is_none() {
                            if let Some(w) = self.project.walls.iter().find(|w| w.id == id) {
                                self.wall_edit_snapshot = Some((id, WallProps {
                                    thickness: w.thickness,
                                    left_side: w.left_side.clone(),
                                    right_side: w.right_side.clone(),
                                }));
                            }
                        }

                        if let Some(wall) = self.project.walls.iter_mut().find(|w| w.id == id) {
                            ui.label("Стена");
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Толщина (мм):");
                                ui.add(
                                    egui::DragValue::new(&mut wall.thickness)
                                        .range(10.0..=1000.0)
                                        .speed(5.0),
                                );
                            });

                            let length_mm = wall.length();
                            let length_label = if length_mm >= 1000.0 {
                                format!("{:.2} м ({:.0} мм)", length_mm / 1000.0, length_mm)
                            } else {
                                format!("{:.0} мм", length_mm)
                            };
                            ui.horizontal(|ui| {
                                ui.label("Длина (графика):");
                                ui.label(length_label);
                            });

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(60, 200, 80), "●");
                                ui.label("Начало");
                                ui.add_space(12.0);
                                ui.colored_label(egui::Color32::from_rgb(230, 210, 50), "●");
                                ui.label("Конец");
                            });

                            ui.add_space(8.0);

                            let left_color = egui::Color32::from_rgb(100, 160, 220);
                            ui.horizontal(|ui| {
                                ui.colored_label(left_color, "■");
                                ui.label("Левая сторона");
                            });
                            ui.indent("left_side", |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Длина (мм):");
                                    ui.add(
                                        egui::DragValue::new(&mut wall.left_side.length)
                                            .range(1.0..=100000.0)
                                            .speed(10.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Высота начала (мм):");
                                    ui.add(
                                        egui::DragValue::new(&mut wall.left_side.height_start)
                                            .range(100.0..=10000.0)
                                            .speed(10.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Высота конца (мм):");
                                    ui.add(
                                        egui::DragValue::new(&mut wall.left_side.height_end)
                                            .range(100.0..=10000.0)
                                            .speed(10.0),
                                    );
                                });
                                let left_area_m2 = wall.left_side.gross_area() / 1_000_000.0;
                                ui.horizontal(|ui| {
                                    ui.label("Площадь:");
                                    ui.label(format!("{:.2} м²", left_area_m2));
                                });
                                Self::show_side_sections(ui, &wall.left_side, "left");
                            });

                            ui.add_space(4.0);

                            let right_color = egui::Color32::from_rgb(170, 100, 200);
                            ui.horizontal(|ui| {
                                ui.colored_label(right_color, "■");
                                ui.label("Правая сторона");
                            });
                            ui.indent("right_side", |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Длина (мм):");
                                    ui.add(
                                        egui::DragValue::new(&mut wall.right_side.length)
                                            .range(1.0..=100000.0)
                                            .speed(10.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Высота начала (мм):");
                                    ui.add(
                                        egui::DragValue::new(&mut wall.right_side.height_start)
                                            .range(100.0..=10000.0)
                                            .speed(10.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Высота конца (мм):");
                                    ui.add(
                                        egui::DragValue::new(&mut wall.right_side.height_end)
                                            .range(100.0..=10000.0)
                                            .speed(10.0),
                                    );
                                });
                                let right_area_m2 = wall.right_side.gross_area() / 1_000_000.0;
                                ui.horizontal(|ui| {
                                    ui.label("Площадь:");
                                    ui.label(format!("{:.2} м²", right_area_m2));
                                });
                                Self::show_side_sections(ui, &wall.right_side, "right");
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.strong("Услуги");
                            ui.add_space(4.0);
                        } else {
                            ui.label("Стена не найдена");
                            self.editor.selection = Selection::None;
                        }

                        if self.project.walls.iter().any(|w| w.id == id) {
                            let left_color = egui::Color32::from_rgb(100, 160, 220);
                            let right_color = egui::Color32::from_rgb(170, 100, 200);
                            self.show_wall_side_services(ui, id, WallSide::Left, "Левая сторона", left_color);
                            ui.add_space(4.0);
                            self.show_wall_side_services(ui, id, WallSide::Right, "Правая сторона", right_color);
                        }
                    }
                    Selection::Opening(id) => {
                        if self.opening_edit_snapshot.is_none() {
                            if let Some(o) = self.project.openings.iter().find(|o| o.id == id) {
                                self.opening_edit_snapshot = Some((id, o.kind.clone()));
                            }
                        }

                        let errors: Vec<&str> = self
                            .project
                            .openings
                            .iter()
                            .find(|o| o.id == id)
                            .map(|o| self.opening_errors(o))
                            .unwrap_or_default();

                        let wall_thickness: Option<f64> = self
                            .project
                            .openings
                            .iter()
                            .find(|o| o.id == id)
                            .and_then(|o| o.wall_id)
                            .and_then(|wid| {
                                self.project.walls.iter().find(|w| w.id == wid)
                            })
                            .map(|w| w.thickness);

                        if let Some(opening) =
                            self.project.openings.iter_mut().find(|o| o.id == id)
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
                                    ui.horizontal(|ui| {
                                        ui.label("Высота (мм):");
                                        ui.add(
                                            egui::DragValue::new(height)
                                                .range(500.0..=3500.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Ширина (мм):");
                                        ui.add(
                                            egui::DragValue::new(width)
                                                .range(300.0..=3000.0)
                                                .speed(10.0),
                                        );
                                    });
                                    if let Some(thick) = wall_thickness {
                                        ui.horizontal(|ui| {
                                            ui.label("Глубина (мм):");
                                            ui.label(format!("{:.0}", thick));
                                        });
                                    }
                                }
                                OpeningKind::Window {
                                    height,
                                    width,
                                    sill_height,
                                    reveal_width,
                                } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Высота (мм):");
                                        ui.add(
                                            egui::DragValue::new(height)
                                                .range(200.0..=3000.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Ширина (мм):");
                                        ui.add(
                                            egui::DragValue::new(width)
                                                .range(200.0..=5000.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Подоконник (мм):");
                                        ui.add(
                                            egui::DragValue::new(sill_height)
                                                .range(0.0..=2500.0)
                                                .speed(10.0),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Откос (мм):");
                                        ui.add(
                                            egui::DragValue::new(reveal_width)
                                                .range(0.0..=500.0)
                                                .speed(5.0),
                                        );
                                    });
                                }
                            }
                        } else {
                            ui.label("Проём не найден");
                            self.editor.selection = Selection::None;
                        }

                        if let Some(opening) = self.project.openings.iter().find(|o| o.id == id) {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.strong("Услуги");
                            ui.add_space(4.0);

                            let svcs = self.project.opening_services.get(&id)
                                .map(|v| v.as_slice()).unwrap_or(&[]);
                            let rows = self.build_assigned_rows_for(svcs, |ut| {
                                self.compute_opening_quantity(ut, opening)
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
                    Selection::Room(id) => {
                        let metrics = self
                            .project
                            .rooms
                            .iter()
                            .find(|r| r.id == id)
                            .and_then(|r| compute_room_metrics(r, &self.project.walls));

                        if let Some(room) =
                            self.project.rooms.iter_mut().find(|r| r.id == id)
                        {
                            ui.label("Комната");
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Название:");
                                if ui.text_edit_singleline(&mut room.name).changed() {
                                    self.dirty = true;
                                }
                            });

                            ui.add_space(4.0);

                            if let Some(m) = &metrics {
                                ui.horizontal(|ui| {
                                    ui.label("Площадь:");
                                    ui.label(format!("{:.2} м²", m.area / 1_000_000.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Периметр:");
                                    ui.label(format!("{:.2} м", m.perimeter / 1000.0));
                                });
                            }

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Стен в контуре:");
                                ui.label(format!("{}", room.wall_ids.len()));
                            });
                        } else {
                            ui.label("Комната не найдена");
                            self.editor.selection = Selection::None;
                        }

                        if let Some(room) = self.project.rooms.iter().find(|r| r.id == id) {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.strong("Услуги");
                            ui.add_space(4.0);

                            let svcs = self.project.room_services.get(&id)
                                .map(|v| v.as_slice()).unwrap_or(&[]);
                            let rows = self.build_assigned_rows_for(svcs, |ut| {
                                self.compute_room_quantity(ut, room)
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
                }
            });
    }
}
