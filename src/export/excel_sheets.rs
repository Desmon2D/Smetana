use rust_xlsxwriter::{Format, Worksheet};
use uuid::Uuid;

use crate::editor::room_metrics::compute_room_metrics;
use crate::model::{AssignedService, OpeningKind, PriceList, Project, UnitType, WallSide,
                   opening_area_mm2, wall_side_quantity, opening_quantity, room_quantity};

pub(super) fn write_rooms_sheet(
    sheet: &mut Worksheet,
    project: &Project,
    fmt_header: &Format,
    fmt_text: &Format,
    fmt_number: &Format,
    fmt_section: &Format,
) -> Result<(), String> {
    // Write header row
    let room_headers = [
        "Помещение",
        "Площадь пола (м²)",
        "Периметр (м)",
        "Площадь стен брутто (м²)",
        "Площадь стен нетто (м²)",
    ];
    for (col, header) in room_headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *header, fmt_header)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
    }

    // Set reasonable column widths
    sheet.set_column_width(0, 20).ok();
    sheet.set_column_width(1, 18).ok();
    sheet.set_column_width(2, 15).ok();
    sheet.set_column_width(3, 24).ok();
    sheet.set_column_width(4, 24).ok();

    // Write summary data rows
    let mut row: u32 = 1;
    for room in &project.rooms {
        let metrics = compute_room_metrics(room, &project.walls);
        let floor_area_m2 = metrics.as_ref().map_or(0.0, |m| m.area / 1e6);
        let perimeter_m = metrics.as_ref().map_or(0.0, |m| m.perimeter / 1e3);

        // Gross wall area: use the room-facing side for each wall
        let mut gross_area_mm2 = 0.0;
        let mut net_area_mm2 = 0.0;
        for (wi, wall_id) in room.wall_ids.iter().enumerate() {
            if let Some(wall) = project.walls.iter().find(|w| w.id == *wall_id) {
                let side = match room.wall_sides[wi] {
                    WallSide::Left => &wall.left_side,
                    WallSide::Right => &wall.right_side,
                };
                let ga = side.gross_area();
                gross_area_mm2 += ga;

                // Subtract openings on this wall
                net_area_mm2 += ga - opening_area_mm2(wall, &project.openings);
            }
        }

        sheet
            .write_string_with_format(row, 0, &room.name, fmt_text)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(row, 1, floor_area_m2, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(row, 2, perimeter_m, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(row, 3, gross_area_mm2 / 1e6, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(row, 4, net_area_mm2 / 1e6, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;

        row += 1;
    }

    // Per-room detail breakdown
    row += 1; // blank row after summary
    for room in &project.rooms {
        // Section header: "Помещение: <name>"
        sheet
            .write_string_with_format(row, 0, &format!("Помещение: {}", room.name), fmt_section)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        row += 1;

        // --- Walls sub-table ---
        sheet
            .write_string_with_format(row, 0, "Стены", fmt_section)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        row += 1;

        let wall_sub_headers = [
            "Стена",
            "Сторона",
            "Высота нач. (мм)",
            "Высота кон. (мм)",
            "Длина (мм)",
            "Толщина (мм)",
            "Площадь брутто (м²)",
            "Площадь нетто (м²)",
        ];
        for (col, header) in wall_sub_headers.iter().enumerate() {
            sheet
                .write_string_with_format(row, col as u16, *header, fmt_header)
                .map_err(|e| format!("Ошибка записи: {e}"))?;
        }
        row += 1;

        for (wi, wall_id) in room.wall_ids.iter().enumerate() {
            if let Some(wall) = project.walls.iter().find(|w| w.id == *wall_id) {
                let side = match room.wall_sides[wi] {
                    WallSide::Left => &wall.left_side,
                    WallSide::Right => &wall.right_side,
                };
                let side_label = match room.wall_sides[wi] {
                    WallSide::Left => "лев.",
                    WallSide::Right => "прав.",
                };
                let label = format!("С{}", wi + 1);
                let ga = side.gross_area();
                let na = ga - opening_area_mm2(wall, &project.openings);

                sheet
                    .write_string_with_format(row, 0, &label, fmt_text)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
                sheet
                    .write_string_with_format(row, 1, side_label, fmt_text)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
                sheet
                    .write_number_with_format(row, 2, side.height_start, fmt_number)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
                sheet
                    .write_number_with_format(row, 3, side.height_end, fmt_number)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
                sheet
                    .write_number_with_format(row, 4, side.length, fmt_number)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
                sheet
                    .write_number_with_format(row, 5, wall.thickness, fmt_number)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
                sheet
                    .write_number_with_format(row, 6, ga / 1e6, fmt_number)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
                sheet
                    .write_number_with_format(row, 7, na / 1e6, fmt_number)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;

                row += 1;
            }
        }

        // --- Windows sub-table ---
        // Collect windows on this room's walls
        let windows: Vec<_> = room
            .wall_ids
            .iter()
            .flat_map(|wid| {
                project
                    .openings
                    .iter()
                    .filter(move |o| o.wall_id == Some(*wid))
                    .filter(|o| matches!(o.kind, OpeningKind::Window { .. }))
            })
            .collect();

        if !windows.is_empty() {
            row += 1; // spacing
            sheet
                .write_string_with_format(row, 0, "Окна", fmt_section)
                .map_err(|e| format!("Ошибка записи: {e}"))?;
            row += 1;

            let win_sub_headers = [
                "Окно",
                "Высота (мм)",
                "Ширина (мм)",
                "Откос (мм)",
                "Высота подоконника (мм)",
                "Периметр откоса (м)",
                "Площадь откоса (м²)",
            ];
            for (col, header) in win_sub_headers.iter().enumerate() {
                sheet
                    .write_string_with_format(row, col as u16, *header, fmt_header)
                    .map_err(|e| format!("Ошибка записи: {e}"))?;
            }
            row += 1;

            for (oi, opening) in windows.iter().enumerate() {
                if let OpeningKind::Window {
                    height,
                    width,
                    sill_height,
                    reveal_width,
                } = &opening.kind
                {
                    let label = format!("О{}", oi + 1);
                    let reveal_perim_mm = 2.0 * height + 2.0 * width;
                    let reveal_area_mm2 = reveal_perim_mm * reveal_width;

                    sheet
                        .write_string_with_format(row, 0, &label, fmt_text)
                        .map_err(|e| format!("Ошибка записи: {e}"))?;
                    sheet
                        .write_number_with_format(row, 1, *height, fmt_number)
                        .map_err(|e| format!("Ошибка записи: {e}"))?;
                    sheet
                        .write_number_with_format(row, 2, *width, fmt_number)
                        .map_err(|e| format!("Ошибка записи: {e}"))?;
                    sheet
                        .write_number_with_format(row, 3, *reveal_width, fmt_number)
                        .map_err(|e| format!("Ошибка записи: {e}"))?;
                    sheet
                        .write_number_with_format(row, 4, *sill_height, fmt_number)
                        .map_err(|e| format!("Ошибка записи: {e}"))?;
                    sheet
                        .write_number_with_format(row, 5, reveal_perim_mm / 1e3, fmt_number)
                        .map_err(|e| format!("Ошибка записи: {e}"))?;
                    sheet
                        .write_number_with_format(row, 6, reveal_area_mm2 / 1e6, fmt_number)
                        .map_err(|e| format!("Ошибка записи: {e}"))?;

                    row += 1;
                }
            }
        }

        row += 1; // spacing between rooms
    }

    Ok(())
}

pub(super) fn write_doors_sheet(
    sheet: &mut Worksheet,
    project: &Project,
    fmt_header: &Format,
    fmt_text: &Format,
    fmt_number: &Format,
) -> Result<(), String> {
    let door_headers = [
        "Дверь",
        "Высота (мм)",
        "Ширина (мм)",
        "Глубина (мм)",
        "Периметр (м)",
        "Из помещения",
        "В помещение",
    ];
    for (col, header) in door_headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *header, fmt_header)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
    }

    sheet.set_column_width(0, 12).ok();
    sheet.set_column_width(1, 14).ok();
    sheet.set_column_width(2, 14).ok();
    sheet.set_column_width(3, 14).ok();
    sheet.set_column_width(4, 14).ok();
    sheet.set_column_width(5, 18).ok();
    sheet.set_column_width(6, 18).ok();

    // Write door data rows
    let mut door_row: u32 = 1;
    let mut door_idx = 0usize;
    for opening in &project.openings {
        let OpeningKind::Door { height, width } = &opening.kind else {
            continue;
        };
        door_idx += 1;
        let label = format!("Д{}", door_idx);

        // Depth = wall thickness
        let depth = opening
            .wall_id
            .and_then(|wid| project.walls.iter().find(|w| w.id == wid))
            .map_or(0.0, |w| w.thickness);

        // Door perimeter: 2×height + width (no threshold)
        let perim_mm = 2.0 * height + width;

        // Find rooms that contain this door's wall
        let rooms_with_wall: Vec<&str> = opening
            .wall_id
            .map(|wid| {
                project
                    .rooms
                    .iter()
                    .filter(|r| r.wall_ids.contains(&wid))
                    .map(|r| r.name.as_str())
                    .collect()
            })
            .unwrap_or_default();

        let from_room = rooms_with_wall.first().copied().unwrap_or("—");
        let to_room = rooms_with_wall.get(1).copied().unwrap_or("—");

        sheet
            .write_string_with_format(door_row, 0, &label, fmt_text)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(door_row, 1, *height, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(door_row, 2, *width, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(door_row, 3, depth, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_number_with_format(door_row, 4, perim_mm / 1e3, fmt_number)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_string_with_format(door_row, 5, from_room, fmt_text)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        sheet
            .write_string_with_format(door_row, 6, to_room, fmt_text)
            .map_err(|e| format!("Ошибка записи: {e}"))?;

        door_row += 1;
    }

    Ok(())
}

pub(super) fn write_estimate_sheet(
    sheet: &mut Worksheet,
    project: &Project,
    price_list: &PriceList,
    fmt_header: &Format,
    fmt_text: &Format,
    fmt_number: &Format,
    fmt_currency: &Format,
) -> Result<(), String> {
    let estimate_headers = [
        "Помещение / Объект",
        "Услуга",
        "Ед. изм.",
        "Количество",
        "Цена за ед. (₽)",
        "Стоимость (₽)",
    ];
    for (col, header) in estimate_headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *header, fmt_header)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
    }

    sheet.set_column_width(0, 25).ok();
    sheet.set_column_width(1, 25).ok();
    sheet.set_column_width(2, 10).ok();
    sheet.set_column_width(3, 12).ok();
    sheet.set_column_width(4, 18).ok();
    sheet.set_column_width(5, 18).ok();

    // Write estimate data grouped by room
    let mut est_row: u32 = 1;
    let mut grand_total = 0.0;

    // Helper to write service rows for an object
    let write_services = |sheet: &mut Worksheet,
                              row: &mut u32,
                              total: &mut f64,
                              label: &str,
                              obj_id: Uuid,
                              wall_side: Option<WallSide>,
                              services: &[AssignedService]|
     -> Result<(), String> {
        for svc in services {
            let tmpl = match price_list.services.iter().find(|s| s.id == svc.service_template_id) {
                Some(t) => t,
                None => continue,
            };
            let price = svc.custom_price.unwrap_or(tmpl.price_per_unit);
            let qty = compute_quantity(project, tmpl.unit_type, obj_id, wall_side);
            let cost = qty * price;
            *total += cost;

            sheet
                .write_string_with_format(*row, 0, label, fmt_text)
                .map_err(|e| format!("Ошибка записи: {e}"))?;
            sheet
                .write_string_with_format(*row, 1, &tmpl.name, fmt_text)
                .map_err(|e| format!("Ошибка записи: {e}"))?;
            sheet
                .write_string_with_format(*row, 2, tmpl.unit_type.label(), fmt_text)
                .map_err(|e| format!("Ошибка записи: {e}"))?;
            sheet
                .write_number_with_format(*row, 3, qty, fmt_number)
                .map_err(|e| format!("Ошибка записи: {e}"))?;
            sheet
                .write_number_with_format(*row, 4, price, fmt_currency)
                .map_err(|e| format!("Ошибка записи: {e}"))?;
            sheet
                .write_number_with_format(*row, 5, cost, fmt_currency)
                .map_err(|e| format!("Ошибка записи: {e}"))?;

            *row += 1;
        }
        Ok(())
    };

    for room in &project.rooms {
        // Room services
        if let Some(svcs) = project.room_services.get(&room.id) {
            if !svcs.is_empty() {
                write_services(
                    sheet,
                    &mut est_row,
                    &mut grand_total,
                    &room.name,
                    room.id,
                    None,
                    svcs,
                )?;
            }
        }

        // Wall services for walls in this room (per-side)
        for (wi, wall_id) in room.wall_ids.iter().enumerate() {
            if let Some(wall_svcs) = project.wall_services.get(wall_id) {
                // Left side services
                for section in &wall_svcs.left.sections {
                    if !section.is_empty() {
                        let label = format!("{} / Стена С{} (лев.)", room.name, wi + 1);
                        write_services(
                            sheet,
                            &mut est_row,
                            &mut grand_total,
                            &label,
                            *wall_id,
                            Some(WallSide::Left),
                            section,
                        )?;
                    }
                }
                // Right side services
                for section in &wall_svcs.right.sections {
                    if !section.is_empty() {
                        let label = format!("{} / Стена С{} (прав.)", room.name, wi + 1);
                        write_services(
                            sheet,
                            &mut est_row,
                            &mut grand_total,
                            &label,
                            *wall_id,
                            Some(WallSide::Right),
                            section,
                        )?;
                    }
                }
            }

            // Opening services for openings on this wall (windows only in room context)
            if let Some(wall) = project.walls.iter().find(|w| w.id == *wall_id) {
                for opening in project
                    .openings
                    .iter()
                    .filter(|o| o.wall_id == Some(wall.id))
                    .filter(|o| matches!(o.kind, OpeningKind::Window { .. }))
                {
                    if let Some(svcs) = project.opening_services.get(&opening.id) {
                        if !svcs.is_empty() {
                            let label = format!("{} / Окно", room.name);
                            write_services(
                                sheet,
                                &mut est_row,
                                &mut grand_total,
                                &label,
                                opening.id,
                                None,
                                svcs,
                            )?;
                        }
                    }
                }
            }
        }
    }

    // Door services (not grouped by room)
    let mut di = 0usize;
    for opening in &project.openings {
        if !matches!(opening.kind, OpeningKind::Door { .. }) {
            continue;
        }
        di += 1;
        if let Some(svcs) = project.opening_services.get(&opening.id) {
            if !svcs.is_empty() {
                let label = format!("Дверь Д{}", di);
                write_services(
                    sheet,
                    &mut est_row,
                    &mut grand_total,
                    &label,
                    opening.id,
                    None,
                    svcs,
                )?;
            }
        }
    }

    // TOTAL row
    est_row += 1;
    let fmt_total = Format::new()
        .set_bold()
        .set_border(rust_xlsxwriter::FormatBorder::Thin);
    let fmt_total_currency = Format::new()
        .set_bold()
        .set_num_format("#,##0.00 ₽")
        .set_border(rust_xlsxwriter::FormatBorder::Thin);
    sheet
        .write_string_with_format(est_row, 4, "ИТОГО:", &fmt_total)
        .map_err(|e| format!("Ошибка записи: {e}"))?;
    sheet
        .write_number_with_format(est_row, 5, grand_total, &fmt_total_currency)
        .map_err(|e| format!("Ошибка записи: {e}"))?;

    Ok(())
}

/// Compute quantity for a service assigned to an object.
/// For wall services, `wall_side` specifies which side's dimensions to use.
fn compute_quantity(project: &Project, unit_type: UnitType, obj_id: Uuid, wall_side: Option<WallSide>) -> f64 {
    if let Some(wall) = project.walls.iter().find(|w| w.id == obj_id) {
        let side = wall_side.unwrap_or(WallSide::Left);
        return wall_side_quantity(unit_type, wall, side, &project.openings);
    }
    if let Some(opening) = project.openings.iter().find(|o| o.id == obj_id) {
        return opening_quantity(unit_type, opening);
    }
    if let Some(room) = project.rooms.iter().find(|r| r.id == obj_id) {
        return room_quantity(unit_type, room, &project.walls);
    }
    if unit_type == UnitType::Piece { 1.0 } else { 0.0 }
}
