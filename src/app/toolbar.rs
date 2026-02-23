use eframe::egui;

use crate::editor::EditorTool;
use crate::export::export_to_xlsx;
use crate::model::ProjectDefaults;
use super::{App, AppScreen};

pub(super) fn show_defaults_form(ui: &mut egui::Ui, defaults: &mut ProjectDefaults) {
    ui.label("Стена:");
    egui::Grid::new("defaults_wall").num_columns(2).show(ui, |ui| {
        ui.label("Толщина (мм):");
        ui.add(egui::DragValue::new(&mut defaults.wall_thickness).range(50.0..=1000.0).speed(10.0));
        ui.end_row();
        ui.label("Высота (мм):");
        ui.add(egui::DragValue::new(&mut defaults.wall_height).range(1000.0..=10000.0).speed(10.0));
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.label("Дверь:");
    egui::Grid::new("defaults_door").num_columns(2).show(ui, |ui| {
        ui.label("Высота (мм):");
        ui.add(egui::DragValue::new(&mut defaults.door_height).range(500.0..=5000.0).speed(10.0));
        ui.end_row();
        ui.label("Ширина (мм):");
        ui.add(egui::DragValue::new(&mut defaults.door_width).range(300.0..=3000.0).speed(10.0));
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.label("Окно:");
    egui::Grid::new("defaults_window").num_columns(2).show(ui, |ui| {
        ui.label("Высота (мм):");
        ui.add(egui::DragValue::new(&mut defaults.window_height).range(200.0..=5000.0).speed(10.0));
        ui.end_row();
        ui.label("Ширина (мм):");
        ui.add(egui::DragValue::new(&mut defaults.window_width).range(200.0..=5000.0).speed(10.0));
        ui.end_row();
        ui.label("Высота подоконника (мм):");
        ui.add(egui::DragValue::new(&mut defaults.window_sill_height).range(0.0..=5000.0).speed(10.0));
        ui.end_row();
        ui.label("Ширина откоса (мм):");
        ui.add(egui::DragValue::new(&mut defaults.window_reveal_width).range(0.0..=1000.0).speed(10.0));
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
            self.flush_property_edits();
            self.history.undo(&mut self.project);
        } else if ctrl_y || ctrl_shift_z {
            self.flush_property_edits();
            self.history.redo(&mut self.project);
        }

        ctx.input(|i| {
            if !i.modifiers.ctrl && !i.modifiers.alt {
                if i.key_pressed(egui::Key::V) {
                    self.set_tool(EditorTool::Select);
                } else if i.key_pressed(egui::Key::W) {
                    self.set_tool(EditorTool::Wall);
                } else if i.key_pressed(egui::Key::D) {
                    self.set_tool(EditorTool::Door);
                } else if i.key_pressed(egui::Key::O) {
                    self.set_tool(EditorTool::Window);
                } else if i.key_pressed(egui::Key::T) {
                    self.set_tool(EditorTool::Label);
                }
            }
        });
    }

    pub(super) fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Инструмент:");

                let prev_tool = self.editor.active_tool;
                let tool = &mut self.editor.active_tool;
                ui.selectable_value(tool, EditorTool::Select, "Выбор (V)");
                ui.selectable_value(tool, EditorTool::Wall, "Стена (W)");
                ui.selectable_value(tool, EditorTool::Door, "Дверь (D)");
                ui.selectable_value(tool, EditorTool::Window, "Окно (O)");
                ui.selectable_value(tool, EditorTool::Label, "Подпись (T)");

                if prev_tool == EditorTool::Wall && self.editor.active_tool != EditorTool::Wall {
                    self.editor.wall_tool.reset();
                }

                ui.separator();

                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("Отменить"))
                    .clicked()
                {
                    self.flush_property_edits();
                    self.history.undo(&mut self.project);
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("Повторить"))
                    .clicked()
                {
                    self.flush_property_edits();
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

                let can_report = !self.has_validation_errors();
                if ui
                    .add_enabled(can_report, egui::Button::new("Сформировать отчёт"))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Сохранить отчёт")
                        .add_filter("Excel", &["xlsx"])
                        .set_file_name(&format!("{}.xlsx", self.project.name))
                        .save_file()
                    {
                        match export_to_xlsx(&self.project, &self.price_list, &path) {
                            Ok(()) => {
                                self.status_message =
                                    Some(format!("Отчёт сохранён: {}", path.display()));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Ошибка: {e}"));
                            }
                        }
                    }
                }

                ui.separator();

                if ui.button("Услуги").clicked() {
                    self.show_price_list_window = !self.show_price_list_window;
                }
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
                        if ui.add_enabled(name_ok, egui::Button::new("Создать")).clicked() {
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
        use crate::editor::Selection;

        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Структура проекта");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Размер подписей:");
                    ui.add(egui::Slider::new(&mut self.label_scale, 0.5..=3.0).step_by(0.1));
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(format!("Стен: {}", self.project.walls.len()));
                    ui.label(format!("Проёмов: {}", self.project.openings.len()));
                    ui.label(format!("Подписей: {}", self.project.labels.len()));

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
            self.dirty = true;
        }
        if !open {
            self.show_project_settings = false;
        }
    }
}
