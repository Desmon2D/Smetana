use eframe::egui;

use crate::editor::Selection;
use crate::editor::room_metrics::compute_room_metrics;
use crate::history::WallProps;
use crate::model::{OpeningKind, TargetObjectType, WallSide};
use super::{App, ServiceTarget};

impl App {
    pub(super) fn show_right_panel(&mut self, ctx: &egui::Context) {
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

                        // Pre-compute junction info before mutable borrow
                        let (left_has_junctions, left_total, right_has_junctions, right_total) =
                            if let Some(w) = self.project.walls.iter().find(|w| w.id == id) {
                                (
                                    !w.left_side.junctions.is_empty(),
                                    w.left_side.computed_total_length(&self.project.walls),
                                    !w.right_side.junctions.is_empty(),
                                    w.right_side.computed_total_length(&self.project.walls),
                                )
                            } else {
                                (false, 0.0, false, 0.0)
                            };

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
                                    if left_has_junctions {
                                        ui.label(format!("{:.0}", left_total));
                                    } else {
                                        ui.add(
                                            egui::DragValue::new(&mut wall.left_side.length)
                                                .range(1.0..=100000.0)
                                                .speed(10.0),
                                        );
                                    }
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
                                Self::show_side_sections(ui, &mut wall.left_side, "left");
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
                                    if right_has_junctions {
                                        ui.label(format!("{:.0}", right_total));
                                    } else {
                                        ui.add(
                                            egui::DragValue::new(&mut wall.right_side.length)
                                                .range(1.0..=100000.0)
                                                .speed(10.0),
                                        );
                                    }
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
                                Self::show_side_sections(ui, &mut wall.right_side, "right");
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
                                    ui.label("Площадь (брутто):");
                                    ui.label(format!("{:.2} м²", m.gross_area / 1_000_000.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Площадь (нетто):");
                                    ui.label(format!("{:.2} м²", m.net_area / 1_000_000.0));
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
                }); // ScrollArea
            });
    }
}
