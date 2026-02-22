use std::collections::HashMap;
use eframe::egui;

use crate::editor::room_detection::compute_room_metrics;
use crate::model::{AssignedService, Opening, OpeningKind, Project, TargetObjectType, UnitType, Wall, WallSide};
use super::{App, ServiceTarget};

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
    pub(super) fn compute_wall_side_quantity(&self, unit_type: UnitType, wall: &Wall, side: WallSide) -> f64 {
        match unit_type {
            UnitType::Piece => 1.0,
            UnitType::SquareMeter => {
                let side_data = match side {
                    WallSide::Left => &wall.left_side,
                    WallSide::Right => &wall.right_side,
                };
                let gross = side_data.gross_area();
                let openings_area: f64 = wall
                    .openings
                    .iter()
                    .filter_map(|oid| self.project.openings.iter().find(|o| o.id == *oid))
                    .map(|o| o.kind.height() * o.kind.width())
                    .sum();
                (gross - openings_area) / 1_000_000.0
            }
            UnitType::LinearMeter => {
                let side_data = match side {
                    WallSide::Left => &wall.left_side,
                    WallSide::Right => &wall.right_side,
                };
                side_data.length / 1000.0
            }
        }
    }

    pub(super) fn compute_wall_section_quantity(&self, unit_type: UnitType, wall: &Wall, side: WallSide, section_index: usize) -> f64 {
        let side_data = match side {
            WallSide::Left => &wall.left_side,
            WallSide::Right => &wall.right_side,
        };

        if let Some(section) = side_data.sections.get(section_index) {
            match unit_type {
                UnitType::Piece => 1.0,
                UnitType::SquareMeter => section.gross_area() / 1_000_000.0,
                UnitType::LinearMeter => section.length / 1000.0,
            }
        } else {
            self.compute_wall_side_quantity(unit_type, wall, side)
        }
    }

    pub(super) fn compute_opening_quantity(&self, unit_type: UnitType, opening: &Opening) -> f64 {
        match unit_type {
            UnitType::Piece => 1.0,
            UnitType::SquareMeter => match &opening.kind {
                OpeningKind::Door { height, width } => height * width / 1_000_000.0,
                OpeningKind::Window { height, width, reveal_width, .. } => {
                    let reveal_perimeter = 2.0 * height + 2.0 * width;
                    reveal_perimeter * reveal_width / 1_000_000.0
                }
            },
            UnitType::LinearMeter => match &opening.kind {
                OpeningKind::Door { height, width } => (2.0 * height + width) / 1000.0,
                OpeningKind::Window { height, width, .. } => {
                    (2.0 * height + 2.0 * width) / 1000.0
                }
            },
        }
    }

    pub(super) fn compute_room_quantity(&self, unit_type: UnitType, room: &crate::model::Room) -> f64 {
        match unit_type {
            UnitType::Piece => 1.0,
            UnitType::SquareMeter => {
                compute_room_metrics(room, &self.project.walls)
                    .map_or(0.0, |m| m.area / 1_000_000.0)
            }
            UnitType::LinearMeter => {
                compute_room_metrics(room, &self.project.walls)
                    .map_or(0.0, |m| m.perimeter / 1000.0)
            }
        }
    }

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
        grid_id: &str,
        rows: &[AssignedServiceRow],
        prices: &mut Vec<f64>,
    ) -> (Option<usize>, Option<usize>) {
        let mut remove_idx: Option<usize> = None;
        let reset_idx: Option<usize> = None;

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

        let _ = (grid_id, &reset_idx);
        (reset_idx, remove_idx)
    }

    pub(super) fn show_wall_side_services(
        &mut self,
        ui: &mut egui::Ui,
        wall_id: uuid::Uuid,
        side: WallSide,
        side_label: &str,
        side_color: egui::Color32,
    ) {
        const SECTION_COLORS: &[(u8, u8, u8)] = &[
            (100, 180, 240),
            (240, 160, 100),
            (100, 220, 140),
            (220, 120, 220),
            (240, 220, 100),
            (120, 220, 220),
        ];

        ui.horizontal(|ui| {
            ui.colored_label(side_color, "■");
            ui.label(side_label);
        });

        let section_count = self.project.walls.iter()
            .find(|w| w.id == wall_id)
            .map(|w| {
                let sd = match side {
                    WallSide::Left => &w.left_side,
                    WallSide::Right => &w.right_side,
                };
                sd.section_count()
            })
            .unwrap_or(1);

        let has_sections = section_count > 1;

        for sec_idx in 0..section_count {
            if has_sections {
                let color_idx = sec_idx % SECTION_COLORS.len();
                let (cr, cg, cb) = SECTION_COLORS[color_idx];
                let color = egui::Color32::from_rgb(cr, cg, cb);
                ui.horizontal(|ui| {
                    ui.colored_label(color, "●");
                    ui.label(format!("Секция {}", sec_idx + 1));
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

            let wall = self.project.walls.iter().find(|w| w.id == wall_id);
            let rows = if let Some(wall) = wall {
                let s = side;
                self.build_assigned_rows_for(&section_snapshot, |ut| {
                    self.compute_wall_section_quantity(ut, wall, s, sec_idx)
                })
            } else {
                Vec::new()
            };
            let mut prices: Vec<f64> = rows.iter().map(|r| r.effective_price).collect();

            let grid_id = format!("wall_{wall_id}_{side:?}_s{sec_idx}_services");
            let (_reset_idx, remove_idx) =
                Self::show_services_list(ui, &grid_id, &rows, &mut prices);

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
                    self.dirty = true;
                }
            }

            {
                let wall_svcs = self.project.wall_services.get_mut(&wall_id);
                if let Some(wall_svcs) = wall_svcs {
                    let side_svcs = match side {
                        WallSide::Left => &mut wall_svcs.left,
                        WallSide::Right => &mut wall_svcs.right,
                    };
                    if let Some(section) = side_svcs.sections.get_mut(sec_idx) {
                        for (i, row) in rows.iter().enumerate() {
                            if !row.valid || i >= section.len() {
                                continue;
                            }
                            let new_price = prices[i];
                            if (new_price - row.template_price).abs() < 0.01 {
                                if section[i].custom_price.is_some() {
                                    section[i].custom_price = None;
                                    self.dirty = true;
                                }
                            } else if (new_price - row.effective_price).abs() > 0.001 {
                                section[i].custom_price = Some(new_price);
                                self.dirty = true;
                            }
                        }
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
        let mut prices: Vec<f64> = rows.iter().map(|r| r.effective_price).collect();
        let grid_id = format!("svc_{obj_id}");
        let (_reset_idx, remove_idx) =
            Self::show_services_list(ui, &grid_id, &rows, &mut prices);

        if ui.small_button("+ Добавить услугу").clicked() {
            self.service_picker_target = Some(target);
            self.show_service_picker = true;
            self.service_picker_filter.clear();
        }

        if let Some(idx) = remove_idx {
            let svcs = services_map(&mut self.project).entry(obj_id).or_default();
            if idx < svcs.len() {
                svcs.remove(idx);
                self.dirty = true;
            }
        }

        {
            let svcs = services_map(&mut self.project).get_mut(&obj_id);
            if let Some(svcs) = svcs {
                for (i, row) in rows.iter().enumerate() {
                    if !row.valid || i >= svcs.len() {
                        continue;
                    }
                    let new_price = prices[i];
                    if (new_price - row.template_price).abs() < 0.01 {
                        if svcs[i].custom_price.is_some() {
                            svcs[i].custom_price = None;
                            self.dirty = true;
                        }
                    } else if (new_price - row.effective_price).abs() > 0.001 {
                        svcs[i].custom_price = Some(new_price);
                        self.dirty = true;
                    }
                }
            }
        }

        let _ = target_type;
    }
}
