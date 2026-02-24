use eframe::egui;

/// A horizontal row with a label and a `DragValue`.  Returns `true` when the
/// value was changed by the user.
pub(super) fn labeled_drag(
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
pub(super) fn labeled_value(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(value);
    });
}

/// A DragValue with override + reset pattern.
/// Shows the override value if set, otherwise the computed value.
/// Returns (Some(new_value) if changed, reset_clicked).
pub(super) fn labeled_drag_override(
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
