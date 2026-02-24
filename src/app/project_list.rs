use eframe::egui;

use super::App;
use super::toolbar::show_defaults_form;
use crate::model::ProjectDefaults;
use crate::persistence::delete_project;

fn format_system_time(t: std::time::SystemTime) -> String {
    match t.duration_since(std::time::SystemTime::UNIX_EPOCH) {
        Ok(dur) => {
            let secs = dur.as_secs();
            let days = secs / 86400;
            let time_of_day = secs % 86400;
            let hours = time_of_day / 3600;
            let minutes = (time_of_day % 3600) / 60;

            // Simple date calculation from days since epoch
            let (year, month, day) = days_to_ymd(days);
            format!(
                "{:02}.{:02}.{} {:02}:{:02}",
                day, month, year, hours, minutes
            )
        }
        Err(_) => "—".to_string(),
    }
}

fn days_to_ymd(days_since_epoch: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

impl App {
    pub(super) fn show_project_list(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.heading("Сметана — Строительная смета");
                ui.add_space(20.0);
            });

            ui.horizontal(|ui| {
                ui.label("Новый проект:");
                ui.text_edit_singleline(&mut self.new_project_name);
                let name_ok = !self.new_project_name.trim().is_empty();
                if ui
                    .add_enabled(name_ok, egui::Button::new("Создать"))
                    .clicked()
                {
                    let name = self.new_project_name.trim().to_string();
                    let defaults = self.new_project_defaults.clone();
                    self.new_project_name.clear();
                    self.new_project_defaults = ProjectDefaults::default();
                    self.create_new_project(name, defaults);
                }
            });

            egui::CollapsingHeader::new("Размеры по умолчанию")
                .default_open(false)
                .show(ui, |ui| {
                    show_defaults_form(ui, &mut self.new_project_defaults);
                });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            if self.project_entries.is_empty() {
                ui.label("Нет сохранённых проектов.");
                return;
            }

            ui.label("Сохранённые проекты:");
            ui.add_space(4.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("project_list_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.strong("Название");
                        ui.strong("Изменён");
                        ui.strong("");
                        ui.end_row();

                        let mut open_idx: Option<usize> = None;

                        for (i, entry) in self.project_entries.iter().enumerate() {
                            let is_selected = self.project_list_selection == Some(i);

                            if ui.selectable_label(is_selected, &entry.name).clicked() {
                                self.project_list_selection = Some(i);
                            }

                            let date_str = format_system_time(entry.modified);
                            ui.label(&date_str);

                            ui.horizontal(|ui| {
                                if ui.button("Открыть").clicked() {
                                    open_idx = Some(i);
                                }
                                if ui.button("Удалить").clicked() {
                                    self.confirm_delete = Some(i);
                                }
                            });

                            ui.end_row();
                        }

                        if let Some(i) = open_idx {
                            let path = self.project_entries[i].path.clone();
                            self.open_project_from_path(&path);
                        }
                    });
            });

            if let Some(sel) = self.project_list_selection
                && ctx.input(|i| i.key_pressed(egui::Key::Enter))
            {
                let path = self.project_entries[sel].path.clone();
                self.open_project_from_path(&path);
            }

            if let Some(del_idx) = self.confirm_delete {
                let name = self.project_entries[del_idx].name.clone();
                let mut open = true;
                egui::Window::new("Подтверждение")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut open)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(format!("Удалить проект «{name}»?"));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Удалить").clicked() {
                                let path = self.project_entries[del_idx].path.clone();
                                let _ = delete_project(&path);
                                self.refresh_project_list();
                            }
                            if ui.button("Отмена").clicked() {
                                self.confirm_delete = None;
                            }
                        });
                    });
                if !open {
                    self.confirm_delete = None;
                }
            }

            if let Some(msg) = &self.status_message {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::RED, msg);
            }
        });
    }
}
