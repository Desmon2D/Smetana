use eframe::egui;

use crate::persistence::delete_project;
use super::App;

enum ProjectListAction {
    Open(usize),
}

fn format_system_time(t: std::time::SystemTime) -> String {
    match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => {
            let secs = dur.as_secs();
            let days = secs / 86400;
            let time_of_day = secs % 86400;
            let hours = time_of_day / 3600;
            let minutes = (time_of_day % 3600) / 60;
            let (year, month, day) = days_to_ymd(days);
            format!("{day:02}.{month:02}.{year} {hours:02}:{minutes:02}")
        }
        Err(_) => "—".to_string(),
    }
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
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
                    self.new_project_name.clear();
                    self.create_new_project(name);
                    return;
                }
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

                        let mut action: Option<ProjectListAction> = None;

                        for (i, entry) in self.project_entries.iter().enumerate() {
                            let is_selected = self.project_list_selection == Some(i);

                            if ui
                                .selectable_label(is_selected, &entry.name)
                                .clicked()
                            {
                                self.project_list_selection = Some(i);
                            }

                            let date_str = format_system_time(entry.modified);
                            ui.label(&date_str);

                            ui.horizontal(|ui| {
                                if ui.button("Открыть").clicked() {
                                    action = Some(ProjectListAction::Open(i));
                                }
                                if ui.button("Удалить").clicked() {
                                    self.confirm_delete = Some(i);
                                }
                            });

                            ui.end_row();
                        }

                        if let Some(a) = action {
                            match a {
                                ProjectListAction::Open(i) => {
                                    let path = self.project_entries[i].path.clone();
                                    self.open_project_from_path(&path);
                                }
                            }
                        }
                    });
            });

            if let Some(sel) = self.project_list_selection {
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let path = self.project_entries[sel].path.clone();
                    self.open_project_from_path(&path);
                }
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
