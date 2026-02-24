use eframe::egui;

use super::{App, property_edits};
use crate::editor::Selection;
use crate::model::OpeningKind;

impl App {
    pub(super) fn show_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Свойства");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| match self.editor.selection {
                    Selection::None => {
                        ui.label("Ничего не выбрано");
                    }
                    Selection::Point(id) => self.show_point_properties(ui, id),
                    Selection::Edge(id) => self.show_edge_properties(ui, id),
                    Selection::Room(id) => self.show_room_properties(ui, id),
                    Selection::Wall(id) => self.show_wall_properties(ui, id),
                    Selection::Opening(id) => self.show_opening_properties(ui, id),
                    Selection::Label(id) => self.show_label_properties(ui, id),
                });
            });
    }

    fn show_point_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        if self.project.point(id).is_none() {
            ui.label("Точка не найдена");
            self.editor.selection = Selection::None;
            return;
        }

        ui.label("Точка");
        ui.add_space(8.0);

        // Take snapshot on first edit
        if self.edit_snapshot_version != Some(self.history.version) {
            self.history.snapshot(&self.project);
            self.edit_snapshot_version = Some(self.history.version);
        }

        let point = self.project.point(id).unwrap();
        let mut x = point.position.x;
        let mut y = point.position.y;
        let mut h = point.height;

        let mut changed = false;
        changed |= property_edits::labeled_drag(ui, "X (мм):", &mut x, -1e6..=1e6, 10.0);
        changed |= property_edits::labeled_drag(ui, "Y (мм):", &mut y, -1e6..=1e6, 10.0);
        changed |= property_edits::labeled_drag(ui, "Высота (мм):", &mut h, 100.0..=10000.0, 10.0);

        if changed && let Some(point) = self.project.point_mut(id) {
            point.position.x = x;
            point.position.y = y;
            point.height = h;
        }

        // "Used in" list
        ui.add_space(8.0);
        ui.separator();
        ui.label("Используется в:");

        let mut refs = Vec::new();
        for room in &self.project.rooms {
            if room.points.contains(&id) || room.cutouts.iter().any(|c| c.contains(&id)) {
                refs.push(format!("Комната: {}", room.name));
            }
        }
        for (i, wall) in self.project.walls.iter().enumerate() {
            if wall.points.contains(&id) {
                refs.push(format!("Стена #{}", i + 1));
            }
        }
        for opening in &self.project.openings {
            if opening.points.contains(&id) {
                let kind_str = match &opening.kind {
                    OpeningKind::Door { .. } => "Дверь",
                    OpeningKind::Window { .. } => "Окно",
                };
                refs.push(kind_str.to_string());
            }
        }

        if refs.is_empty() {
            ui.label("(не используется)");
        } else {
            for r in &refs {
                ui.label(r);
            }
        }
    }

    fn show_edge_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        // Compute read-only values first (before mutable borrow)
        let edge_info = self.project.edge(id).map(|e| {
            let point_a = e.point_a;
            let point_b = e.point_b;
            let dist_override = e.distance_override;
            let angle_override = e.angle_override;

            // Computed distance from coordinates
            let computed_dist = {
                let a = self.project.point(point_a);
                let b = self.project.point(point_b);
                match (a, b) {
                    (Some(a), Some(b)) => a.position.distance(b.position),
                    _ => 0.0,
                }
            };

            let effective_dist = dist_override.unwrap_or(computed_dist);

            // Heights at endpoints
            let height_a = self.project.point(point_a).map(|p| p.height).unwrap_or(0.0);
            let height_b = self.project.point(point_b).map(|p| p.height).unwrap_or(0.0);
            let avg_height = (height_a + height_b) / 2.0;
            let wall_area_m2 = effective_dist * avg_height / 1_000_000.0;

            (
                computed_dist,
                dist_override,
                angle_override,
                height_a,
                height_b,
                wall_area_m2,
            )
        });

        let Some((computed_dist, dist_override, angle_override, height_a, height_b, wall_area_m2)) =
            edge_info
        else {
            ui.label("Ребро не найдено");
            self.editor.selection = Selection::None;
            return;
        };

        ui.label("Ребро");
        ui.add_space(8.0);

        // Take snapshot on first edit
        if self.edit_snapshot_version != Some(self.history.version) {
            self.history.snapshot(&self.project);
            self.edit_snapshot_version = Some(self.history.version);
        }

        // Distance override
        let (dist_changed, dist_reset) = property_edits::labeled_drag_override(
            ui,
            "Расстояние (мм):",
            dist_override,
            computed_dist,
            1.0..=1e6,
            10.0,
        );
        if let Some(new_val) = dist_changed
            && let Some(edge) = self.project.edge_mut(id)
        {
            edge.distance_override = Some(new_val);
        }
        if dist_reset && let Some(edge) = self.project.edge_mut(id) {
            edge.distance_override = None;
        }

        property_edits::labeled_value(ui, "Вычисленное:", format!("{:.0} мм", computed_dist));

        ui.add_space(4.0);

        // Angle override
        let (angle_changed, angle_reset) = property_edits::labeled_drag_override(
            ui,
            "Угол (°):",
            angle_override,
            0.0, // no computed angle without context
            0.0..=360.0,
            1.0,
        );
        if let Some(new_val) = angle_changed
            && let Some(edge) = self.project.edge_mut(id)
        {
            edge.angle_override = Some(new_val);
        }
        if angle_reset && let Some(edge) = self.project.edge_mut(id) {
            edge.angle_override = None;
        }

        ui.add_space(4.0);
        ui.separator();
        property_edits::labeled_value(ui, "Высота в A:", format!("{:.0} мм", height_a));
        property_edits::labeled_value(ui, "Высота в B:", format!("{:.0} мм", height_b));
        property_edits::labeled_value(ui, "Площадь стены:", format!("{:.3} м²", wall_area_m2));
    }

    fn show_room_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        // Compute read-only values first
        let metrics = self.project.room(id).map(|r| {
            let area_m2 = r.floor_area(&self.project) / 1_000_000.0;
            let perimeter_m = r.perimeter(&self.project) / 1000.0;
            let point_count = r.points.len();
            let cutout_count = r.cutouts.len();
            (area_m2, perimeter_m, point_count, cutout_count)
        });

        if let Some(room) = self.project.rooms.iter_mut().find(|r| r.id == id) {
            ui.label("Комната");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Название:");
                if ui.text_edit_singleline(&mut room.name).changed() {
                    self.history.mark_dirty();
                }
            });

            ui.add_space(4.0);

            if let Some((area_m2, perimeter_m, point_count, cutout_count)) = metrics {
                property_edits::labeled_value(ui, "Площадь пола:", format!("{:.3} м²", area_m2));
                property_edits::labeled_value(ui, "Периметр:", format!("{:.3} м", perimeter_m));
                property_edits::labeled_value(ui, "Точек:", format!("{}", point_count));
                property_edits::labeled_value(ui, "Вырезов:", format!("{}", cutout_count));
            }

            ui.add_space(8.0);

            // Add Cutout button
            if ui.button("Добавить вырез").clicked() {
                self.editor.tool_state.building_cutout = true;
                self.editor.tool_state.points.clear();
                self.editor.active_tool = crate::editor::Tool::Room;
            }

            if ui.button("Удалить комнату").clicked() {
                self.history.snapshot(&self.project);
                self.project.remove_room(id);
                self.editor.selection = Selection::None;
            }
        } else {
            ui.label("Комната не найдена");
            self.editor.selection = Selection::None;
        }
    }

    fn show_wall_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let point_count = match self.project.wall(id) {
            Some(w) => w.points.len(),
            None => {
                ui.label("Стена не найдена");
                self.editor.selection = Selection::None;
                return;
            }
        };

        ui.label("Стена");
        ui.add_space(8.0);

        // Take snapshot on first edit
        if self.edit_snapshot_version != Some(self.history.version) {
            self.history.snapshot(&self.project);
            self.edit_snapshot_version = Some(self.history.version);
        }

        // Read current color
        let wall = self.project.wall(id).unwrap();
        let mut color = egui::Color32::from_rgba_premultiplied(
            wall.color[0],
            wall.color[1],
            wall.color[2],
            wall.color[3],
        );

        ui.horizontal(|ui| {
            ui.label("Цвет:");
            if ui.color_edit_button_srgba(&mut color).changed()
                && let Some(wall) = self.project.walls.iter_mut().find(|w| w.id == id)
            {
                wall.color = [color.r(), color.g(), color.b(), color.a()];
            }
        });

        property_edits::labeled_value(ui, "Точек:", format!("{}", point_count));
    }

    fn show_opening_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let (kind_label, point_count) = match self.project.opening(id) {
            Some(o) => {
                let label = match &o.kind {
                    OpeningKind::Door { .. } => "Дверь",
                    OpeningKind::Window { .. } => "Окно",
                };
                (label, o.points.len())
            }
            None => {
                ui.label("Проём не найден");
                self.editor.selection = Selection::None;
                return;
            }
        };

        ui.label(kind_label);
        ui.add_space(8.0);

        // Take snapshot on first edit
        if self.edit_snapshot_version != Some(self.history.version) {
            self.history.snapshot(&self.project);
            self.edit_snapshot_version = Some(self.history.version);
        }

        if let Some(opening) = self.project.opening_mut(id) {
            match &mut opening.kind {
                OpeningKind::Door { height, width } => {
                    property_edits::labeled_drag(ui, "Высота (мм):", height, 500.0..=3500.0, 10.0);
                    property_edits::labeled_drag(ui, "Ширина (мм):", width, 300.0..=3000.0, 10.0);
                }
                OpeningKind::Window {
                    height,
                    width,
                    sill_height,
                    reveal_width,
                } => {
                    property_edits::labeled_drag(ui, "Высота (мм):", height, 200.0..=3000.0, 10.0);
                    property_edits::labeled_drag(ui, "Ширина (мм):", width, 200.0..=5000.0, 10.0);
                    property_edits::labeled_drag(
                        ui,
                        "Подоконник (мм):",
                        sill_height,
                        0.0..=2500.0,
                        10.0,
                    );
                    property_edits::labeled_drag(ui, "Откос (мм):", reveal_width, 0.0..=500.0, 5.0);
                }
            }
        }

        property_edits::labeled_value(ui, "Точек:", format!("{}", point_count));
    }

    fn show_label_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        // Auto-delete label if text was emptied
        let is_empty = self
            .project
            .labels
            .iter()
            .find(|l| l.id == id)
            .is_some_and(|l| l.text.trim().is_empty());
        if is_empty {
            self.project.labels.retain(|l| l.id != id);
            self.editor.selection = Selection::None;
            return;
        }

        if let Some(label) = self.project.labels.iter_mut().find(|l| l.id == id) {
            ui.label("Подпись");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Текст:");
                if ui.text_edit_singleline(&mut label.text).changed() {
                    self.history.mark_dirty();
                }
            });

            property_edits::labeled_drag(
                ui,
                "Размер шрифта:",
                &mut label.font_size,
                6.0..=72.0,
                0.5,
            );

            let mut rotation_deg = label.rotation.to_degrees();
            if property_edits::labeled_drag(ui, "Поворот (°):", &mut rotation_deg, 0.0..=360.0, 1.0)
            {
                label.rotation = rotation_deg.to_radians();
            }
        } else {
            ui.label("Подпись не найдена");
            self.editor.selection = Selection::None;
        }
    }
}
