use eframe::egui;

use crate::model::{AssignedService, OpeningKind, TargetObjectType, WallSide};
use super::{App, ServiceTarget};

impl App {
    pub(super) fn show_service_picker_window(&mut self, ctx: &egui::Context) {
        if !self.show_service_picker {
            return;
        }
        let target_type = match &self.service_picker_target {
            Some(ServiceTarget::WallSide { .. }) => Some(TargetObjectType::Wall),
            Some(ServiceTarget::Opening { opening_id }) => {
                self.project.openings.iter().find(|o| o.id == *opening_id).map(|o| match &o.kind {
                    OpeningKind::Door { .. } => TargetObjectType::Door,
                    OpeningKind::Window { .. } => TargetObjectType::Window,
                })
            }
            Some(ServiceTarget::Room { .. }) => Some(TargetObjectType::Room),
            None => None,
        };

        let mut open = true;
        let mut picked_id: Option<uuid::Uuid> = None;

        egui::Window::new("Выбор услуги")
            .open(&mut open)
            .default_size([400.0, 300.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Фильтр:");
                    ui.text_edit_singleline(&mut self.service_picker_filter);
                });
                ui.add_space(4.0);

                let filter_lower = self.service_picker_filter.to_lowercase();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for svc in &self.price_list.services {
                        if let Some(tt) = target_type {
                            if svc.target_type != tt {
                                continue;
                            }
                        }
                        if !filter_lower.is_empty()
                            && !svc.name.to_lowercase().contains(&filter_lower)
                        {
                            continue;
                        }

                        let label = format!(
                            "{} | {} | {:.0} ₽",
                            svc.name,
                            svc.unit_type.label(),
                            svc.price_per_unit
                        );
                        if ui.selectable_label(false, label).clicked() {
                            picked_id = Some(svc.id);
                        }
                    }
                });
            });

        if !open {
            self.show_service_picker = false;
            self.service_picker_target = None;
        }

        if let Some(tmpl_id) = picked_id {
            let new_svc = AssignedService {
                service_template_id: tmpl_id,
                custom_price: None,
            };
            match self.service_picker_target.take() {
                Some(ServiceTarget::WallSide { wall_id, side, section_index }) => {
                    let wall_svcs = self.project.wall_services.entry(wall_id).or_default();
                    let side_svcs = match side {
                        WallSide::Left => &mut wall_svcs.left,
                        WallSide::Right => &mut wall_svcs.right,
                    };
                    side_svcs.ensure_section(section_index).push(new_svc);
                    self.dirty = true;
                }
                Some(ServiceTarget::Opening { opening_id }) => {
                    self.project
                        .opening_services
                        .entry(opening_id)
                        .or_default()
                        .push(new_svc);
                    self.dirty = true;
                }
                Some(ServiceTarget::Room { room_id }) => {
                    self.project
                        .room_services
                        .entry(room_id)
                        .or_default()
                        .push(new_svc);
                    self.dirty = true;
                }
                None => {}
            }
            self.show_service_picker = false;
        }
    }
}
