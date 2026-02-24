use eframe::egui;

use super::{App, AppScreen, Selection, Tool, VisibilityMode};
use crate::model::{OpeningKind, ProjectDefaults};

// ---------------------------------------------------------------------------
// Property edit helpers
// ---------------------------------------------------------------------------

/// A horizontal row with a label and a `DragValue`. Returns `true` when changed.
fn labeled_drag(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui
            .add(egui::DragValue::new(val).range(range).speed(speed))
            .changed();
    });
    changed
}

/// A horizontal row with a label and a read-only value string.
fn labeled_value(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(value);
    });
}

/// A DragValue with override + reset pattern.
/// Returns (Some(new_value) if changed, reset_clicked).
fn labeled_drag_override(
    ui: &mut egui::Ui,
    label: &str,
    current_override: Option<f64>,
    computed_value: f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
) -> (Option<f64>, bool) {
    let mut new_val = None;
    let mut reset = false;

    ui.horizontal(|ui| {
        ui.label(label);
        let mut val = current_override.unwrap_or(computed_value);
        let resp = ui.add(egui::DragValue::new(&mut val).range(range).speed(speed));
        if resp.changed() {
            new_val = Some(val);
        }
        if current_override.is_some() && ui.small_button("Сброс").clicked() {
            reset = true;
        }
    });

    (new_val, reset)
}

// ---------------------------------------------------------------------------
// Defaults form (shared by toolbar dialog and project list)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Toolbar + keyboard shortcuts
// ---------------------------------------------------------------------------

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

                let mut tool = self.active_tool;
                ui.selectable_value(&mut tool, Tool::Select, "Выбор (V)");
                ui.selectable_value(&mut tool, Tool::Point, "Точка (P)");
                ui.selectable_value(&mut tool, Tool::Room, "Комната (R)");
                ui.selectable_value(&mut tool, Tool::Wall, "Стена (W)");
                ui.selectable_value(&mut tool, Tool::Door, "Дверь (D)");
                ui.selectable_value(&mut tool, Tool::Window, "Окно (O)");
                ui.selectable_value(&mut tool, Tool::Label, "Подпись (T)");
                self.set_tool(tool);

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
                            self.close_new_project_form();
                            self.create_new_project(name, defaults);
                        }
                        if ui.button("Отмена").clicked() {
                            self.close_new_project_form();
                        }
                    });
                });
            if !open {
                self.close_new_project_form();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Left panel
    // -----------------------------------------------------------------------

    pub(super) fn show_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Структура проекта");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Режим:");
                    let vis = &mut self.visibility;
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

                    let mut clicked_room = None;
                    for room in &self.project.rooms {
                        let is_selected = self.selection == Selection::Room(room.id);
                        let label = egui::SelectableLabel::new(is_selected, &room.name);
                        if ui.add(label).clicked() {
                            clicked_room = Some(room.id);
                        }
                    }

                    if let Some(id) = clicked_room {
                        self.selection = Selection::Room(id);
                    }

                    // Labels list
                    if !self.project.labels.is_empty() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.label(format!("Подписи ({})", self.project.labels.len()));
                        ui.add_space(4.0);

                        let mut clicked_label = None;
                        for label in &self.project.labels {
                            let is_selected = self.selection == Selection::Label(label.id);
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
                            self.selection = Selection::Label(id);
                        }
                    }
                });
            });
    }

    // -----------------------------------------------------------------------
    // Project settings window
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Right panel (properties)
    // -----------------------------------------------------------------------

    pub(super) fn show_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Свойства");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| match self.selection {
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
            self.selection = Selection::None;
            return;
        }

        ui.label("Точка");
        ui.add_space(8.0);

        self.ensure_edit_snapshot();

        let point = self.project.point(id).unwrap();
        let mut x = point.position.x;
        let mut y = point.position.y;
        let mut h = point.height;

        let mut changed = false;
        changed |= labeled_drag(ui, "X (мм):", &mut x, -1e6..=1e6, 10.0);
        changed |= labeled_drag(ui, "Y (мм):", &mut y, -1e6..=1e6, 10.0);
        changed |= labeled_drag(ui, "Высота (мм):", &mut h, 100.0..=10000.0, 10.0);

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
        let edge_info = self.project.edge(id).map(|e| {
            let point_a = e.point_a;
            let point_b = e.point_b;
            let dist_override = e.distance_override;
            let angle_override = e.angle_override;

            let computed_dist = {
                let a = self.project.point(point_a);
                let b = self.project.point(point_b);
                match (a, b) {
                    (Some(a), Some(b)) => a.position.distance(b.position),
                    _ => 0.0,
                }
            };

            let effective_dist = dist_override.unwrap_or(computed_dist);
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
            self.selection = Selection::None;
            return;
        };

        ui.label("Ребро");
        ui.add_space(8.0);

        self.ensure_edit_snapshot();

        // Distance override
        let (dist_changed, dist_reset) = labeled_drag_override(
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

        labeled_value(ui, "Вычисленное:", format!("{:.0} мм", computed_dist));

        ui.add_space(4.0);

        // Angle override
        let (angle_changed, angle_reset) = labeled_drag_override(
            ui,
            "Угол (°):",
            angle_override,
            0.0,
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
        labeled_value(ui, "Высота в A:", format!("{:.0} мм", height_a));
        labeled_value(ui, "Высота в B:", format!("{:.0} мм", height_b));
        labeled_value(ui, "Площадь стены:", format!("{:.3} м²", wall_area_m2));
    }

    fn show_room_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
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
                labeled_value(ui, "Площадь пола:", format!("{:.3} м²", area_m2));
                labeled_value(ui, "Периметр:", format!("{:.3} м", perimeter_m));
                labeled_value(ui, "Точек:", format!("{}", point_count));
                labeled_value(ui, "Вырезов:", format!("{}", cutout_count));
            }

            ui.add_space(8.0);

            if ui.button("Добавить вырез").clicked() {
                self.tool_state.building_cutout = true;
                self.tool_state.points.clear();
                self.active_tool = Tool::Room;
            }

            if ui.button("Удалить комнату").clicked() {
                self.history.snapshot(&self.project);
                self.project.remove_room(id);
                self.selection = Selection::None;
            }
        } else {
            ui.label("Комната не найдена");
            self.selection = Selection::None;
        }
    }

    fn show_wall_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let point_count = match self.project.wall(id) {
            Some(w) => w.points.len(),
            None => {
                ui.label("Стена не найдена");
                self.selection = Selection::None;
                return;
            }
        };

        ui.label("Стена");
        ui.add_space(8.0);

        self.ensure_edit_snapshot();

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

        labeled_value(ui, "Точек:", format!("{}", point_count));
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
                self.selection = Selection::None;
                return;
            }
        };

        ui.label(kind_label);
        ui.add_space(8.0);

        self.ensure_edit_snapshot();

        if let Some(opening) = self.project.opening_mut(id) {
            match &mut opening.kind {
                OpeningKind::Door { height, width } => {
                    labeled_drag(ui, "Высота (мм):", height, 500.0..=3500.0, 10.0);
                    labeled_drag(ui, "Ширина (мм):", width, 300.0..=3000.0, 10.0);
                }
                OpeningKind::Window {
                    height,
                    width,
                    sill_height,
                    reveal_width,
                } => {
                    labeled_drag(ui, "Высота (мм):", height, 200.0..=3000.0, 10.0);
                    labeled_drag(ui, "Ширина (мм):", width, 200.0..=5000.0, 10.0);
                    labeled_drag(
                        ui,
                        "Подоконник (мм):",
                        sill_height,
                        0.0..=2500.0,
                        10.0,
                    );
                    labeled_drag(ui, "Откос (мм):", reveal_width, 0.0..=500.0, 5.0);
                }
            }
        }

        labeled_value(ui, "Точек:", format!("{}", point_count));
    }

    fn show_label_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let is_empty = self
            .project
            .labels
            .iter()
            .find(|l| l.id == id)
            .is_some_and(|l| l.text.trim().is_empty());
        if is_empty {
            self.project.labels.retain(|l| l.id != id);
            self.selection = Selection::None;
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

            labeled_drag(
                ui,
                "Размер шрифта:",
                &mut label.font_size,
                6.0..=72.0,
                0.5,
            );

            let mut rotation_deg = label.rotation.to_degrees();
            if labeled_drag(ui, "Поворот (°):", &mut rotation_deg, 0.0..=360.0, 1.0) {
                label.rotation = rotation_deg.to_radians();
            }
        } else {
            ui.label("Подпись не найдена");
            self.selection = Selection::None;
        }
    }
}
