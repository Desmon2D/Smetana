use eframe::egui;

use crate::model::{ServiceTemplate, TargetObjectType, UnitType};
use crate::persistence::{load_price_list, save_price_list_to};
use super::App;

impl App {
    pub(super) fn show_price_list_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_price_list_window {
            return;
        }
        let mut open = self.show_price_list_window;
        egui::Window::new("Список услуг")
            .open(&mut open)
            .default_size([550.0, 400.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Добавить").clicked() {
                        self.price_list.services.push(ServiceTemplate::new(
                            "Новая услуга".to_string(),
                            UnitType::SquareMeter,
                            0.0,
                            TargetObjectType::Wall,
                        ));
                        self.selected_service_idx = Some(self.price_list.services.len() - 1);
                    }

                    let can_delete = self
                        .selected_service_idx
                        .map_or(false, |i| i < self.price_list.services.len());
                    if ui
                        .add_enabled(can_delete, egui::Button::new("Удалить"))
                        .clicked()
                    {
                        if let Some(idx) = self.selected_service_idx {
                            self.price_list.services.remove(idx);
                            if self.price_list.services.is_empty() {
                                self.selected_service_idx = None;
                            } else {
                                self.selected_service_idx =
                                    Some(idx.min(self.price_list.services.len() - 1));
                            }
                        }
                    }

                    ui.separator();

                    if ui.button("Импорт").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Импорт прайс-листа")
                            .pick_file()
                        {
                            match load_price_list(&path) {
                                Ok(pl) => {
                                    self.price_list = pl;
                                    self.selected_service_idx = None;
                                    self.status_message =
                                        Some("Прайс-лист загружен".to_string());
                                }
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Ошибка импорта: {e}"));
                                }
                            }
                        }
                    }

                    if ui.button("Экспорт").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Экспорт прайс-листа")
                            .set_file_name(&format!("{}.json", self.price_list.name))
                            .save_file()
                        {
                            match save_price_list_to(&self.price_list, &path) {
                                Ok(()) => {
                                    self.status_message =
                                        Some("Прайс-лист сохранён".to_string());
                                }
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Ошибка экспорта: {e}"));
                                }
                            }
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Фильтр:");
                    ui.text_edit_singleline(&mut self.price_list_filter);
                });

                ui.add_space(4.0);

                if self.price_list.services.is_empty() {
                    ui.label("Нет услуг. Нажмите «Добавить».");
                    return;
                }

                let filter_lower = self.price_list_filter.to_lowercase();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("price_list_grid")
                        .num_columns(5)
                        .striped(true)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.strong("");
                            ui.strong("Название");
                            ui.strong("Объект");
                            ui.strong("Ед. изм.");
                            ui.strong("Цена за ед.");
                            ui.end_row();

                            let mut new_sel = self.selected_service_idx;

                            for (i, svc) in self.price_list.services.iter_mut().enumerate() {
                                if !filter_lower.is_empty()
                                    && !svc.name.to_lowercase().contains(&filter_lower)
                                {
                                    continue;
                                }

                                let is_selected = self.selected_service_idx == Some(i);

                                if ui
                                    .selectable_label(is_selected, format!("{}", i + 1))
                                    .clicked()
                                {
                                    new_sel = Some(i);
                                }

                                ui.add(
                                    egui::TextEdit::singleline(&mut svc.name)
                                        .desired_width(180.0),
                                );

                                egui::ComboBox::from_id_salt(format!("target_{i}"))
                                    .selected_text(svc.target_type.label())
                                    .width(90.0)
                                    .show_ui(ui, |ui| {
                                        for tt in TargetObjectType::ALL {
                                            ui.selectable_value(
                                                &mut svc.target_type,
                                                tt,
                                                tt.label(),
                                            );
                                        }
                                    });

                                egui::ComboBox::from_id_salt(format!("unit_{i}"))
                                    .selected_text(svc.unit_type.label())
                                    .width(60.0)
                                    .show_ui(ui, |ui| {
                                        for ut in UnitType::ALL {
                                            ui.selectable_value(
                                                &mut svc.unit_type,
                                                ut,
                                                ut.label(),
                                            );
                                        }
                                    });

                                ui.add(
                                    egui::DragValue::new(&mut svc.price_per_unit)
                                        .range(0.0..=f64::MAX)
                                        .speed(10.0)
                                        .suffix(" ₽"),
                                );

                                ui.end_row();
                            }

                            self.selected_service_idx = new_sel;
                        });
                });
            });
        self.show_price_list_window = open;
    }
}
