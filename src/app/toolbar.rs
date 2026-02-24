use eframe::egui;

use super::{App, AppScreen};
use crate::editor::{Selection, Tool, VisibilityMode};
use crate::model::ProjectDefaults;

pub(super) fn show_defaults_form(ui: &mut egui::Ui, defaults: &mut ProjectDefaults) {
    ui.label("Точка:");
    egui::Grid::new("defaults_point")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Высота (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.point_height)
                    .range(1000.0..=10000.0)
                    .speed(10.0),
            );
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.label("Дверь:");
    egui::Grid::new("defaults_door")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Высота (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.door_height)
                    .range(500.0..=5000.0)
                    .speed(10.0),
            );
            ui.end_row();
            ui.label("Ширина (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.door_width)
                    .range(300.0..=3000.0)
                    .speed(10.0),
            );
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.label("Окно:");
    egui::Grid::new("defaults_window")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Высота (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.window_height)
                    .range(200.0..=5000.0)
                    .speed(10.0),
            );
            ui.end_row();
            ui.label("Ширина (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.window_width)
                    .range(200.0..=5000.0)
                    .speed(10.0),
            );
            ui.end_row();
            ui.label("Высота подоконника (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.window_sill_height)
                    .range(0.0..=5000.0)
                    .speed(10.0),
            );
            ui.end_row();
            ui.label("Ширина откоса (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.window_reveal_width)
                    .range(0.0..=1000.0)
                    .speed(10.0),
            );
            ui.end_row();
        });
}

impl App {
    pub(super) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let (ctrl_z, ctrl_y, ctrl_shift_z, ctrl_n, ctrl_o, ctrl_s) = ctx.input(|i| {
            (
                i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift,
                i.modifiers.ctrl && i.key_pressed(egui::Key::Y),
                i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z),
                i.modifiers.ctrl && i.key_pressed(egui::Key::N),
                i.modifiers.ctrl && i.key_pressed(egui::Key::O),
                i.modifiers.ctrl && i.key_pressed(egui::Key::S),
            )
        });

        if ctrl_s {
            self.save_current_project();
        } else if ctrl_n {
            self.show_new_project_dialog = true;
        } else if ctrl_o {
            self.refresh_project_list();
            self.screen = AppScreen::ProjectList;
        } else if ctrl_z {
            self.edit_snapshot_version = None;
            self.history.undo(&mut self.project);
        } else if ctrl_y || ctrl_shift_z {
            self.edit_snapshot_version = None;
            self.history.redo(&mut self.project);
        }

        ctx.input(|i| {
            if !i.modifiers.ctrl && !i.modifiers.alt {
                if i.key_pressed(egui::Key::V) {
                    self.set_tool(Tool::Select);
                } else if i.key_pressed(egui::Key::P) {
                    self.set_tool(Tool::Point);
                } else if i.key_pressed(egui::Key::R) {
                    self.set_tool(Tool::Room);
                } else if i.key_pressed(egui::Key::W) {
                    self.set_tool(Tool::Wall);
                } else if i.key_pressed(egui::Key::D) {
                    self.set_tool(Tool::Door);
                } else if i.key_pressed(egui::Key::O) {
                    self.set_tool(Tool::Window);
                } else if i.key_pressed(egui::Key::T) {
                    self.set_tool(Tool::Label);
                }
            }
        });
    }

    pub(super) fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Инструмент:");

                let tool = &mut self.editor.active_tool;
                let prev_tool = *tool;
                ui.selectable_value(tool, Tool::Select, "Выбор (V)");
                ui.selectable_value(tool, Tool::Point, "Точка (P)");
                ui.selectable_value(tool, Tool::Room, "Комната (R)");
                ui.selectable_value(tool, Tool::Wall, "Стена (W)");
                ui.selectable_value(tool, Tool::Door, "Дверь (D)");
                ui.selectable_value(tool, Tool::Window, "Окно (O)");
                ui.selectable_value(tool, Tool::Label, "Подпись (T)");

                // Clear tool states on toolbar switch
                if *tool != prev_tool {
                    self.editor.room_tool.points.clear();
                    self.editor.room_tool.building_cutout = false;
                    self.editor.polygon_tool.points.clear();
                }

                ui.separator();

                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("Отменить"))
                    .clicked()
                {
                    self.edit_snapshot_version = None;
                    self.history.undo(&mut self.project);
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("Повторить"))
                    .clicked()
                {
                    self.edit_snapshot_version = None;
                    self.history.redo(&mut self.project);
                }

                ui.separator();

                if ui.button("Новый проект").clicked() {
                    self.show_new_project_dialog = true;
                }
                if ui.button("Открыть").clicked() {
                    self.refresh_project_list();
                    self.screen = AppScreen::ProjectList;
                }
                if ui.button("Сохранить").clicked() {
                    self.save_current_project();
                }

                ui.separator();

                if ui.button("Настройки").clicked() {
                    self.show_project_settings = !self.show_project_settings;
                }

                if let Some(msg) = &self.status_message {
                    ui.separator();
                    ui.label(msg);
                }
            });
        });

        if self.show_new_project_dialog {
            let mut open = true;
            egui::Window::new("Новый проект")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Название:");
                        ui.text_edit_singleline(&mut self.new_project_name);
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label("Размеры по умолчанию:");
                    ui.add_space(4.0);
                    show_defaults_form(ui, &mut self.new_project_defaults);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let name_ok = !self.new_project_name.trim().is_empty();
                        if ui
                            .add_enabled(name_ok, egui::Button::new("Создать"))
                            .clicked()
                        {
                            let name = self.new_project_name.trim().to_string();
                            let defaults = self.new_project_defaults.clone();
                            self.new_project_name.clear();
                            self.new_project_defaults = ProjectDefaults::default();
                            self.show_new_project_dialog = false;
                            self.create_new_project(name, defaults);
                        }
                        if ui.button("Отмена").clicked() {
                            self.new_project_name.clear();
                            self.new_project_defaults = ProjectDefaults::default();
                            self.show_new_project_dialog = false;
                        }
                    });
                });
            if !open {
                self.new_project_name.clear();
                self.new_project_defaults = ProjectDefaults::default();
                self.show_new_project_dialog = false;
            }
        }
    }

    pub(super) fn show_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Структура проекта");
                ui.separator();

                // Visibility mode toggle
                ui.horizontal(|ui| {
                    ui.label("Режим:");
                    let vis = &mut self.editor.visibility;
                    ui.selectable_value(vis, VisibilityMode::All, "Всё");
                    ui.selectable_value(vis, VisibilityMode::Rooms, "Комнаты");
                    ui.selectable_value(vis, VisibilityMode::Wireframe, "Каркас");
                });
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Размер подписей:");
                    ui.add(egui::Slider::new(&mut self.label_scale, 0.5..=3.0).step_by(0.1));
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Object counts
                    ui.label(format!("Точек: {}", self.project.points.len()));
                    ui.label(format!("Рёбер: {}", self.project.edges.len()));
                    ui.label(format!("Стен: {}", self.project.walls.len()));
                    ui.label(format!("Проёмов: {}", self.project.openings.len()));
                    ui.label(format!("Подписей: {}", self.project.labels.len()));

                    // Rooms list
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(format!("Комнаты ({})", self.project.rooms.len()));
                    ui.add_space(4.0);

                    let selected_room = match self.editor.selection {
                        Selection::Room(id) => Some(id),
                        _ => None,
                    };

                    let mut clicked_room = None;
                    for room in &self.project.rooms {
                        let is_selected = selected_room == Some(room.id);
                        let label = egui::SelectableLabel::new(is_selected, &room.name);
                        if ui.add(label).clicked() {
                            clicked_room = Some(room.id);
                        }
                    }

                    if let Some(id) = clicked_room {
                        self.editor.selection = Selection::Room(id);
                    }

                    // Labels list
                    if !self.project.labels.is_empty() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.label(format!("Подписи ({})", self.project.labels.len()));
                        ui.add_space(4.0);

                        let selected_label = match self.editor.selection {
                            Selection::Label(id) => Some(id),
                            _ => None,
                        };

                        let mut clicked_label = None;
                        for label in &self.project.labels {
                            let is_selected = selected_label == Some(label.id);
                            let display = if label.text.is_empty() {
                                "(пусто)".to_string()
                            } else {
                                label.text.clone()
                            };
                            let lbl = egui::SelectableLabel::new(is_selected, &display);
                            if ui.add(lbl).clicked() {
                                clicked_label = Some(label.id);
                            }
                        }

                        if let Some(id) = clicked_label {
                            self.editor.selection = Selection::Label(id);
                        }
                    }
                });
            });
    }

    pub(super) fn show_project_settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_project_settings {
            return;
        }
        let mut open = true;
        let before = self.project.defaults.clone();
        egui::Window::new("Настройки проекта")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Размеры по умолчанию для новых элементов:");
                ui.add_space(4.0);
                show_defaults_form(ui, &mut self.project.defaults);
            });
        if self.project.defaults != before {
            self.history.mark_dirty();
        }
        if !open {
            self.show_project_settings = false;
        }
    }
}
