use std::collections::HashMap;
use eframe::egui;

use crate::model::{AssignedService, Project, TargetObjectType, UnitType, WallSide};
use super::{App, SECTION_COLORS, ServiceTarget};

/// Synchronise `custom_price` on each `AssignedService` with the price the
/// user edited in the UI row. Returns `true` if any service was modified.
fn sync_custom_prices(
    svcs: &mut [AssignedService],
    rows: &[AssignedServiceRow],
    prices: &[f64],
) -> bool {
    let mut changed = false;
    for (i, row) in rows.iter().enumerate() {
        if !row.valid || i >= svcs.len() {
            continue;
        }
        let new_price = prices[i];
        if (new_price - row.template_price).abs() < 0.01 {
            if svcs[i].custom_price.is_some() {
                svcs[i].custom_price = None;
                changed = true;
            }
        } else if (new_price - row.effective_price).abs() > 0.001 {
            svcs[i].custom_price = Some(new_price);
            changed = true;
        }
    }
    changed
}

pub(super) struct AssignedServiceRow {
    pub name: String,
    pub unit_label: String,
    pub template_price: f64,
    pub effective_price: f64,
    pub has_custom: bool,
    pub qty: f64,
    pub valid: bool,
}

impl App {
    pub(super) fn build_assigned_rows_for(
        &self,
        assigned: &[AssignedService],
        qty_fn: impl Fn(UnitType) -> f64,
    ) -> Vec<AssignedServiceRow> {
        assigned
            .iter()
            .map(|a| {
                let tmpl = self
                    .price_list
                    .services
                    .iter()
                    .find(|s| s.id == a.service_template_id);
                match tmpl {
                    Some(t) => {
                        let effective = a.custom_price.unwrap_or(t.price_per_unit);
                        let qty = qty_fn(t.unit_type);
                        AssignedServiceRow {
                            name: t.name.clone(),
                            unit_label: t.unit_type.label().to_string(),
                            template_price: t.price_per_unit,
                            effective_price: effective,
                            has_custom: a.custom_price.is_some(),
                            qty,
                            valid: true,
                        }
                    }
                    None => AssignedServiceRow {
                        name: "⚠ Услуга удалена".to_string(),
                        unit_label: "—".to_string(),
                        template_price: 0.0,
                        effective_price: 0.0,
                        has_custom: false,
                        qty: 0.0,
                        valid: false,
                    },
                }
            })
            .collect()
    }

    pub(super) fn show_services_list(
        ui: &mut egui::Ui,
        rows: &[AssignedServiceRow],
        prices: &[f64],
    ) -> Option<usize> {
        let mut remove_idx: Option<usize> = None;

        for (i, row) in rows.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(&row.name);
                if row.valid {
                    ui.label(format!("{:.2} {}", row.qty, row.unit_label));
                    ui.label(format!("{:.0}₽", prices[i]));
                    let cost = row.qty * prices[i];
                    ui.label(format!("= {:.0}₽", cost));
                }
                if ui.small_button("✕").on_hover_text("Убрать услугу").clicked() {
                    remove_idx = Some(i);
                }
            });
        }

        remove_idx
    }

    pub(super) fn show_wall_side_services(
        &mut self,
        ui: &mut egui::Ui,
        wall_id: uuid::Uuid,
        side: WallSide,
        side_label: &str,
        color_offset: usize,
    ) {
        ui.label(side_label);

        let section_count = self.project.wall(wall_id)
            .map(|w| {
                let sd = match side {
                    WallSide::Left => &w.left_side,
                    WallSide::Right => &w.right_side,
                };
                sd.section_count()
            })
            .unwrap_or(1);

        for sec_idx in 0..section_count {
            {
                let global_idx = color_offset + sec_idx;
                let color_idx = global_idx % SECTION_COLORS.len();
                let (cr, cg, cb) = SECTION_COLORS[color_idx];
                let color = egui::Color32::from_rgb(cr, cg, cb);
                ui.horizontal(|ui| {
                    ui.colored_label(color, "●");
                    ui.label(format!("Секция {}", global_idx + 1));
                });
            }

            let section_snapshot: Vec<AssignedService> = self
                .project
                .wall_services
                .get(&wall_id)
                .and_then(|ws| {
                    let ss = match side {
                        WallSide::Left => &ws.left,
                        WallSide::Right => &ws.right,
                    };
                    ss.sections.get(sec_idx)
                })
                .cloned()
                .unwrap_or_default();

            let wall = self.project.wall(wall_id);
            let rows = if let Some(wall) = wall {
                let s = side;
                let openings = &self.project.openings;
                self.build_assigned_rows_for(&section_snapshot, |ut| {
                    crate::model::wall_section_quantity(ut, wall, s, sec_idx, openings)
                })
            } else {
                Vec::new()
            };
            let prices: Vec<f64> = rows.iter().map(|r| r.effective_price).collect();

            let remove_idx =
                Self::show_services_list(ui, &rows, &prices);

            if ui.small_button("+ Добавить услугу").clicked() {
                self.service_picker_target = Some(ServiceTarget::WallSide {
                    wall_id,
                    side,
                    section_index: sec_idx,
                });
                self.show_service_picker = true;
                self.service_picker_filter.clear();
            }

            if let Some(idx) = remove_idx {
                let wall_svcs = self.project.wall_services.entry(wall_id).or_default();
                let side_svcs = match side {
                    WallSide::Left => &mut wall_svcs.left,
                    WallSide::Right => &mut wall_svcs.right,
                };
                let section = side_svcs.ensure_section(sec_idx);
                if idx < section.len() {
                    section.remove(idx);
                    self.history.mark_dirty();
                }
            }

            if let Some(wall_svcs) = self.project.wall_services.get_mut(&wall_id) {
                let side_svcs = match side {
                    WallSide::Left => &mut wall_svcs.left,
                    WallSide::Right => &mut wall_svcs.right,
                };
                if let Some(section) = side_svcs.sections.get_mut(sec_idx) {
                    if sync_custom_prices(section, &rows, &prices) {
                        self.history.mark_dirty();
                    }
                }
            }
        }
    }

    pub(super) fn show_flat_services(
        &mut self,
        ui: &mut egui::Ui,
        obj_id: uuid::Uuid,
        target: ServiceTarget,
        target_type: TargetObjectType,
        rows: Vec<AssignedServiceRow>,
        services_map: fn(&mut Project) -> &mut HashMap<uuid::Uuid, Vec<AssignedService>>,
    ) {
        let prices: Vec<f64> = rows.iter().map(|r| r.effective_price).collect();
        let remove_idx =
            Self::show_services_list(ui, &rows, &prices);

        if ui.small_button("+ Добавить услугу").clicked() {
            self.service_picker_target = Some(target);
            self.show_service_picker = true;
            self.service_picker_filter.clear();
        }

        if let Some(idx) = remove_idx {
            let svcs = services_map(&mut self.project).entry(obj_id).or_default();
            if idx < svcs.len() {
                svcs.remove(idx);
                self.history.mark_dirty();
            }
        }

        if let Some(svcs) = services_map(&mut self.project).get_mut(&obj_id) {
            if sync_custom_prices(svcs, &rows, &prices) {
                self.history.mark_dirty();
            }
        }

        let _ = target_type;
    }
}
