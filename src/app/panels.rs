use eframe::egui;

use super::{App, AppScreen, Selection, Tool};
use super::viewport::VisibilityMode;
use crate::model::{ArrowMode, LinePattern, OpeningKind, ProjectDefaults};

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
            ui.label("Откос (мм):");
            ui.add(
                egui::DragValue::new(&mut defaults.door_reveal_width)
                    .range(0.0..=2000.0)
                    .speed(5.0),
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
    ui.add_space(4.0);
    ui.label("Цвета по умолчанию:");
    egui::Grid::new("defaults_colors")
        .num_columns(2)
        .show(ui, |ui| {
            let mut wall_c = egui::Color32::from_rgba_premultiplied(
                defaults.wall_color[0], defaults.wall_color[1],
                defaults.wall_color[2], defaults.wall_color[3],
            );
            ui.label("Стены:");
            if ui.color_edit_button_srgba(&mut wall_c).changed() {
                defaults.wall_color = [wall_c.r(), wall_c.g(), wall_c.b(), wall_c.a()];
            }
            ui.end_row();

            let mut door_c = egui::Color32::from_rgba_premultiplied(
                defaults.door_color[0], defaults.door_color[1],
                defaults.door_color[2], defaults.door_color[3],
            );
            ui.label("Двери:");
            if ui.color_edit_button_srgba(&mut door_c).changed() {
                defaults.door_color = [door_c.r(), door_c.g(), door_c.b(), door_c.a()];
            }
            ui.end_row();

            let mut window_c = egui::Color32::from_rgba_premultiplied(
                defaults.window_color[0], defaults.window_color[1],
                defaults.window_color[2], defaults.window_color[3],
            );
            ui.label("Окна:");
            if ui.color_edit_button_srgba(&mut window_c).changed() {
                defaults.window_color = [window_c.r(), window_c.g(), window_c.b(), window_c.a()];
            }
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
            self.validate_selection();
        } else if ctrl_y || ctrl_shift_z {
            self.edit_snapshot_version = None;
            self.history.redo(&mut self.project);
            self.validate_selection();
        }

        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if !i.modifiers.ctrl && !i.modifiers.alt {
                    if i.key_pressed(egui::Key::Num1) {
                        self.set_tool(Tool::Select);
                    } else if i.key_pressed(egui::Key::Num2) {
                        self.set_tool(Tool::Point);
                    } else if i.key_pressed(egui::Key::Num3) {
                        self.set_tool(Tool::Edge);
                    } else if i.key_pressed(egui::Key::Num4) {
                        self.set_tool(Tool::Cutout);
                    } else if i.key_pressed(egui::Key::Num5) {
                        self.set_tool(Tool::Room);
                    } else if i.key_pressed(egui::Key::Num6) {
                        self.set_tool(Tool::Door);
                    } else if i.key_pressed(egui::Key::Num7) {
                        self.set_tool(Tool::Window);
                    } else if i.key_pressed(egui::Key::Num8) {
                        self.set_tool(Tool::Wall);
                    } else if i.key_pressed(egui::Key::Num9) {
                        self.set_tool(Tool::Label);
                    }
                }
            });
        }
    }

    pub(super) fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Инструмент:");

                let mut tool = self.active_tool;
                ui.selectable_value(&mut tool, Tool::Select, "Выбор (1)");
                ui.selectable_value(&mut tool, Tool::Point, "Точка (2)");
                ui.selectable_value(&mut tool, Tool::Edge, "Ребро (3)");
                ui.selectable_value(&mut tool, Tool::Cutout, "Вырез (4)");
                ui.selectable_value(&mut tool, Tool::Room, "Комната (5)");
                ui.selectable_value(&mut tool, Tool::Door, "Дверь (6)");
                ui.selectable_value(&mut tool, Tool::Window, "Окно (7)");
                ui.selectable_value(&mut tool, Tool::Wall, "Стена (8)");
                ui.selectable_value(&mut tool, Tool::Label, "Подпись (9)");
                self.set_tool(tool);

                ui.separator();

                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("Отменить"))
                    .clicked()
                {
                    self.edit_snapshot_version = None;
                    self.history.undo(&mut self.project);
                    self.validate_selection();
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("Повторить"))
                    .clicked()
                {
                    self.edit_snapshot_version = None;
                    self.history.redo(&mut self.project);
                    self.validate_selection();
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

                if let Some((msg, created)) = &self.status_message
                    && created.elapsed().as_secs() < 5
                {
                    ui.separator();
                    ui.label(msg.as_str());
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
            let wall_area_gross = effective_dist * avg_height / 1_000_000.0;
            let openings_area = self.project.openings_area_on_edge(point_a, point_b) / 1_000_000.0;
            let wall_area_net = (wall_area_gross - openings_area).max(0.0);

            (
                computed_dist,
                dist_override,
                angle_override,
                height_a,
                height_b,
                wall_area_gross,
                wall_area_net,
            )
        });

        let Some((computed_dist, dist_override, angle_override, height_a, height_b, wall_area_gross, wall_area_net)) =
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

        labeled_value(ui, "Вычисленное:", format!("{:.1} мм", computed_dist));

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
        ui.label("Подпись:");
        ui.horizontal(|ui| {
            if ui.button("Сторона").clicked()
                && let Some(edge) = self.project.edge_mut(id)
            {
                edge.label_flip_side = !edge.label_flip_side;
            }
            if ui.button("Перевернуть").clicked()
                && let Some(edge) = self.project.edge_mut(id)
            {
                edge.label_flip_text = !edge.label_flip_text;
            }
        });
        let mut label_visible = self.project.edge(id).is_some_and(|e| !e.label_hidden);
        if ui.checkbox(&mut label_visible, "Показать подпись").changed()
            && let Some(edge) = self.project.edge_mut(id)
        {
            edge.label_hidden = !label_visible;
        }

        ui.add_space(4.0);

        // Edge line style
        ui.label("Стиль линии:");
        let cur_pattern = self.project.edge(id).map(|e| e.line_pattern).unwrap_or_default();
        egui::ComboBox::from_id_salt("edge_pattern")
            .selected_text(cur_pattern.label())
            .show_ui(ui, |ui| {
                for &p in LinePattern::ALL {
                    if ui.selectable_value(&mut self.project.edge_mut(id).unwrap().line_pattern, p, p.label()).changed() {
                        self.history.mark_dirty();
                    }
                }
            });

        let cur_arrow = self.project.edge(id).map(|e| e.arrow_mode).unwrap_or_default();
        egui::ComboBox::from_id_salt("edge_arrow")
            .selected_text(cur_arrow.label())
            .show_ui(ui, |ui| {
                for &a in ArrowMode::ALL {
                    if ui.selectable_value(&mut self.project.edge_mut(id).unwrap().arrow_mode, a, a.label()).changed() {
                        self.history.mark_dirty();
                    }
                }
            });

        ui.add_space(4.0);
        ui.separator();
        labeled_value(ui, "Высота в A:", format!("{:.1} мм", height_a));
        labeled_value(ui, "Высота в B:", format!("{:.1} мм", height_b));
        labeled_value(ui, "Площадь общая:", format!("{:.4} м²", wall_area_gross));
        labeled_value(ui, "Площадь чистая:", format!("{:.4} м²", wall_area_net));
    }

    fn show_room_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let Some(room) = self.project.room(id) else {
            ui.label("Комната не найдена");
            self.selection = Selection::None;
            return;
        };
        let area_m2 = room.floor_area(&self.project) / 1_000_000.0;
        let perimeter_m = room.perimeter(&self.project) / 1000.0;
        let point_count = room.points.len();
        let cutout_count = room.cutouts.len();
        let current_color = room.color;
        let mut color = egui::Color32::from_rgba_premultiplied(
            current_color[0],
            current_color[1],
            current_color[2],
            current_color[3],
        );

        ui.label("Комната");
        ui.add_space(8.0);

        self.ensure_edit_snapshot();

        let Some(room) = self.project.room_mut(id) else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label("Название:");
            if ui.text_edit_singleline(&mut room.name).changed() {
                self.history.mark_dirty();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Цвет:");
            if ui.color_edit_button_srgba(&mut color).changed()
                && let Some(room) = self.project.room_mut(id)
            {
                room.color = [color.r(), color.g(), color.b(), color.a()];
            }
        });

        ui.horizontal(|ui| {
            if ui.small_button("Копировать цвет").clicked() {
                self.copied_color = Some(current_color);
            }
            if let Some(cc) = self.copied_color
                && ui.small_button("Вставить цвет").clicked()
                && let Some(room) = self.project.room_mut(id)
            {
                room.color = cc;
            }
        });

        ui.add_space(4.0);

        labeled_value(ui, "Площадь пола:", format!("{:.4} м²", area_m2));
        labeled_value(ui, "Периметр:", format!("{:.4} м", perimeter_m));
        labeled_value(ui, "Точек:", format!("{}", point_count));
        labeled_value(ui, "Вырезов:", format!("{}", cutout_count));

        ui.add_space(4.0);

        {
            let has_offset = self.project.room(id).is_some_and(|r| r.name_offset.is_some());
            if has_offset && ui.button("Сбросить положение подписи").clicked() {
                self.ensure_edit_snapshot();
                if let Some(room) = self.project.room_mut(id) {
                    room.name_offset = None;
                }
            }
        }

        ui.add_space(8.0);

        if ui.button("Удалить комнату").clicked() {
            self.history.snapshot(&self.project);
            self.project.remove_room(id);
            self.selection = Selection::None;
        }
    }

    fn show_wall_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let Some(wall) = self.project.wall(id) else {
            ui.label("Стена не найдена");
            self.selection = Selection::None;
            return;
        };
        let point_count = wall.points.len();
        let current_color = wall.color;
        let mut color = egui::Color32::from_rgba_premultiplied(
            current_color[0],
            current_color[1],
            current_color[2],
            current_color[3],
        );

        ui.label("Стена");
        ui.add_space(8.0);

        self.ensure_edit_snapshot();

        ui.horizontal(|ui| {
            ui.label("Цвет:");
            if ui.color_edit_button_srgba(&mut color).changed()
                && let Some(wall) = self.project.wall_mut(id)
            {
                wall.color = [color.r(), color.g(), color.b(), color.a()];
            }
        });

        ui.horizontal(|ui| {
            if ui.small_button("Копировать цвет").clicked() {
                self.copied_color = Some(current_color);
            }
            if let Some(cc) = self.copied_color
                && ui.small_button("Вставить цвет").clicked()
                && let Some(wall) = self.project.wall_mut(id)
            {
                wall.color = cc;
            }
        });

        labeled_value(ui, "Точек:", format!("{}", point_count));
    }

    fn show_opening_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        let (kind_label, point_count, current_color) = match self.project.opening(id) {
            Some(o) => {
                let label = match &o.kind {
                    OpeningKind::Door { .. } => "Дверь",
                    OpeningKind::Window { .. } => "Окно",
                };
                (label, o.points.len(), o.color)
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

        let mut color = egui::Color32::from_rgba_premultiplied(
            current_color[0],
            current_color[1],
            current_color[2],
            current_color[3],
        );
        ui.horizontal(|ui| {
            ui.label("Цвет:");
            if ui.color_edit_button_srgba(&mut color).changed()
                && let Some(opening) = self.project.opening_mut(id)
            {
                opening.color = [color.r(), color.g(), color.b(), color.a()];
            }
        });

        ui.horizontal(|ui| {
            if ui.small_button("Копировать цвет").clicked() {
                self.copied_color = Some(current_color);
            }
            if let Some(cc) = self.copied_color
                && ui.small_button("Вставить цвет").clicked()
                && let Some(opening) = self.project.opening_mut(id)
            {
                opening.color = cc;
            }
        });

        if let Some(opening) = self.project.opening_mut(id) {
            match &mut opening.kind {
                OpeningKind::Door {
                    height,
                    width,
                    reveal_width,
                    swing_edge,
                    swing_outward,
                    swing_mirrored,
                    show_swing,
                } => {
                    labeled_drag(ui, "Высота (мм):", height, 500.0..=3500.0, 10.0);
                    labeled_drag(ui, "Ширина (мм):", width, 300.0..=3000.0, 10.0);
                    labeled_drag(ui, "Откос (мм):", reveal_width, 0.0..=2000.0, 5.0);
                    let reveal_perimeter = *height * 2.0 + *width;
                    let reveal_area = reveal_perimeter * *reveal_width;
                    labeled_value(
                        ui,
                        "Периметр откоса:",
                        format!("{:.3} м", reveal_perimeter / 1000.0),
                    );
                    labeled_value(
                        ui,
                        "Площадь откоса:",
                        format!("{:.3} м²", reveal_area / 1_000_000.0),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Направление").clicked() {
                            *swing_outward = !*swing_outward;
                        }
                        if ui.button("Отразить").clicked() {
                            *swing_mirrored = !*swing_mirrored;
                        }
                        if ui.button("Грань").clicked() {
                            *swing_edge = (*swing_edge + 1) % point_count.max(1);
                        }
                    });
                    ui.checkbox(show_swing, "Показать открывание");
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
                    labeled_drag(ui, "Откос (мм):", reveal_width, 0.0..=2000.0, 5.0);
                    let reveal_perimeter = *height * 2.0 + *width;
                    let reveal_area = reveal_perimeter * *reveal_width;
                    labeled_value(
                        ui,
                        "Периметр откоса:",
                        format!("{:.3} м", reveal_perimeter / 1000.0),
                    );
                    labeled_value(
                        ui,
                        "Площадь откоса:",
                        format!("{:.3} м²", reveal_area / 1_000_000.0),
                    );
                }
            }
        }

        labeled_value(ui, "Точек:", format!("{}", point_count));
    }

    fn show_label_properties(&mut self, ui: &mut egui::Ui, id: uuid::Uuid) {
        if self
            .project
            .label(id)
            .is_some_and(|l| l.text.trim().is_empty())
        {
            self.project.remove_label(id);
            self.selection = Selection::None;
            return;
        }

        let Some(label) = self.project.label_mut(id) else {
            ui.label("Подпись не найдена");
            self.selection = Selection::None;
            return;
        };

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
    }
}
