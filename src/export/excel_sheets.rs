use rust_xlsxwriter::Worksheet;
use uuid::Uuid;

use crate::model::room_metrics::compute_room_metrics;
use crate::model::{AssignedService, OpeningKind, PriceList, Project, Room, WallSide,
                   opening_area_mm2, wall_section_quantity, compute_object_quantity};
use super::excel::{ExcelFormats, write_str, write_num, write_header_row};

// ---------------------------------------------------------------------------
// Sheet 1: Помещения (Rooms)
// ---------------------------------------------------------------------------

pub(super) fn write_rooms_sheet(
    sheet: &mut Worksheet,
    project: &Project,
    fmts: &ExcelFormats,
) -> Result<(), String> {
    // Set reasonable column widths
    sheet.set_column_width(0, 20).ok();
    sheet.set_column_width(1, 18).ok();
    sheet.set_column_width(2, 15).ok();
    sheet.set_column_width(3, 24).ok();
    sheet.set_column_width(4, 24).ok();

    // Summary table
    let mut row = write_rooms_summary(sheet, project, fmts, 0)?;

    // Per-room detail breakdown
    row += 1; // blank row after summary
    for room in &project.rooms {
        // Section header: "Помещение: <name>"
        write_str(sheet, row, 0, &format!("Помещение: {}", room.name), &fmts.section)?;
        row += 1;

        // Walls sub-table
        row = write_room_walls_detail(sheet, room, project, fmts, row)?;

        // Windows sub-table
        row = write_room_windows_detail(sheet, room, project, fmts, row)?;

        row += 1; // spacing between rooms
    }

    Ok(())
}

/// Write the rooms summary table (header + one row per room).
/// Returns the next row after the summary.
fn write_rooms_summary(
    sheet: &mut Worksheet,
    project: &Project,
    fmts: &ExcelFormats,
    start_row: u32,
) -> Result<u32, String> {
    let headers = [
        "Помещение",
        "Площадь пола (м²)",
        "Периметр (м)",
        "Площадь стен брутто (м²)",
        "Площадь стен нетто (м²)",
    ];
    write_header_row(sheet, start_row, &headers, &fmts.header)?;

    let mut row = start_row + 1;
    for room in &project.rooms {
        let metrics = compute_room_metrics(room, &project.walls);
        let floor_area_m2 = metrics.as_ref().map_or(0.0, |m| m.net_area / 1e6);
        let perimeter_m = metrics.as_ref().map_or(0.0, |m| m.perimeter / 1e3);

        // Gross wall area: use the room-facing side for each wall
        let mut gross_area_mm2 = 0.0;
        let mut net_area_mm2 = 0.0;
        for (wi, wall_id) in room.wall_ids.iter().enumerate() {
            if let Some(wall) = project.wall(*wall_id) {
                let side = match room.wall_sides[wi] {
                    WallSide::Left => &wall.left_side,
                    WallSide::Right => &wall.right_side,
                };
                let ga = side.gross_area();
                gross_area_mm2 += ga;
                net_area_mm2 += ga - opening_area_mm2(wall, &project.openings);
            }
        }

        write_str(sheet, row, 0, &room.name, &fmts.text)?;
        write_num(sheet, row, 1, floor_area_m2, &fmts.number)?;
        write_num(sheet, row, 2, perimeter_m, &fmts.number)?;
        write_num(sheet, row, 3, gross_area_mm2 / 1e6, &fmts.number)?;
        write_num(sheet, row, 4, net_area_mm2 / 1e6, &fmts.number)?;

        row += 1;
    }

    Ok(row)
}

/// Write the wall detail sub-table for a single room.
/// Returns the next row after the sub-table.
fn write_room_walls_detail(
    sheet: &mut Worksheet,
    room: &Room,
    project: &Project,
    fmts: &ExcelFormats,
    start_row: u32,
) -> Result<u32, String> {
    let mut row = start_row;

    write_str(sheet, row, 0, "Стены", &fmts.section)?;
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
    write_header_row(sheet, row, &wall_sub_headers, &fmts.header)?;
    row += 1;

    for (wi, wall_id) in room.wall_ids.iter().enumerate() {
        if let Some(wall) = project.wall(*wall_id) {
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

            write_str(sheet, row, 0, &label, &fmts.text)?;
            write_str(sheet, row, 1, side_label, &fmts.text)?;
            write_num(sheet, row, 2, side.height_start, &fmts.number)?;
            write_num(sheet, row, 3, side.height_end, &fmts.number)?;
            write_num(sheet, row, 4, side.length, &fmts.number)?;
            write_num(sheet, row, 5, wall.thickness, &fmts.number)?;
            write_num(sheet, row, 6, ga / 1e6, &fmts.number)?;
            write_num(sheet, row, 7, na / 1e6, &fmts.number)?;

            row += 1;
        }
    }

    Ok(row)
}

/// Write the window detail sub-table for a single room.
/// Returns the next row after the sub-table.
fn write_room_windows_detail(
    sheet: &mut Worksheet,
    room: &Room,
    project: &Project,
    fmts: &ExcelFormats,
    start_row: u32,
) -> Result<u32, String> {
    let mut row = start_row;

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

    if windows.is_empty() {
        return Ok(row);
    }

    row += 1; // spacing
    write_str(sheet, row, 0, "Окна", &fmts.section)?;
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
    write_header_row(sheet, row, &win_sub_headers, &fmts.header)?;
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

            write_str(sheet, row, 0, &label, &fmts.text)?;
            write_num(sheet, row, 1, *height, &fmts.number)?;
            write_num(sheet, row, 2, *width, &fmts.number)?;
            write_num(sheet, row, 3, *reveal_width, &fmts.number)?;
            write_num(sheet, row, 4, *sill_height, &fmts.number)?;
            write_num(sheet, row, 5, reveal_perim_mm / 1e3, &fmts.number)?;
            write_num(sheet, row, 6, reveal_area_mm2 / 1e6, &fmts.number)?;

            row += 1;
        }
    }

    Ok(row)
}

// ---------------------------------------------------------------------------
// Sheet 2: Двери (Doors)
// ---------------------------------------------------------------------------

pub(super) fn write_doors_sheet(
    sheet: &mut Worksheet,
    project: &Project,
    fmts: &ExcelFormats,
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
    write_header_row(sheet, 0, &door_headers, &fmts.header)?;

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
            .and_then(|wid| project.wall(wid))
            .map_or(0.0, |w| w.thickness);

        // Door perimeter: 2*height + width (no threshold)
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

        let from_room = rooms_with_wall.first().copied().unwrap_or("\u{2014}");
        let to_room = rooms_with_wall.get(1).copied().unwrap_or("\u{2014}");

        write_str(sheet, door_row, 0, &label, &fmts.text)?;
        write_num(sheet, door_row, 1, *height, &fmts.number)?;
        write_num(sheet, door_row, 2, *width, &fmts.number)?;
        write_num(sheet, door_row, 3, depth, &fmts.number)?;
        write_num(sheet, door_row, 4, perim_mm / 1e3, &fmts.number)?;
        write_str(sheet, door_row, 5, from_room, &fmts.text)?;
        write_str(sheet, door_row, 6, to_room, &fmts.text)?;

        door_row += 1;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Sheet 3: Смета (Estimate)
// ---------------------------------------------------------------------------

pub(super) fn write_estimate_sheet(
    sheet: &mut Worksheet,
    project: &Project,
    price_list: &PriceList,
    fmts: &ExcelFormats,
) -> Result<(), String> {
    let estimate_headers = [
        "Помещение / Объект",
        "Услуга",
        "Ед. изм.",
        "Количество",
        "Цена за ед. (₽)",
        "Стоимость (₽)",
    ];
    write_header_row(sheet, 0, &estimate_headers, &fmts.header)?;

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
                              section_index: Option<usize>,
                              services: &[AssignedService]|
     -> Result<(), String> {
        for svc in services {
            let tmpl = match price_list.services.iter().find(|s| s.id == svc.service_template_id) {
                Some(t) => t,
                None => continue,
            };
            let price = svc.custom_price.unwrap_or(tmpl.price_per_unit);
            let qty = match (wall_side, section_index) {
                (Some(side), Some(si)) => {
                    // Per-section wall service: use section quantity
                    if let Some(wall) = project.wall(obj_id) {
                        wall_section_quantity(tmpl.unit_type, wall, side, si, &project.openings)
                    } else {
                        compute_object_quantity(project, tmpl.unit_type, obj_id, Some(side))
                    }
                }
                _ => compute_object_quantity(project, tmpl.unit_type, obj_id, wall_side),
            };
            let cost = qty * price;
            *total += cost;

            write_str(sheet, *row, 0, label, &fmts.text)?;
            write_str(sheet, *row, 1, &tmpl.name, &fmts.text)?;
            write_str(sheet, *row, 2, tmpl.unit_type.label(), &fmts.text)?;
            write_num(sheet, *row, 3, qty, &fmts.number)?;
            write_num(sheet, *row, 4, price, &fmts.currency)?;
            write_num(sheet, *row, 5, cost, &fmts.currency)?;

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
                    None,
                    svcs,
                )?;
            }
        }

        // Wall services for walls in this room (per-side, per-section)
        for (wi, wall_id) in room.wall_ids.iter().enumerate() {
            if let Some(wall_svcs) = project.wall_services.get(wall_id) {
                // Left side services
                for (si, section) in wall_svcs.left.sections.iter().enumerate() {
                    if !section.is_empty() {
                        let label = format!("{} / Стена С{} (лев.)", room.name, wi + 1);
                        write_services(
                            sheet,
                            &mut est_row,
                            &mut grand_total,
                            &label,
                            *wall_id,
                            Some(WallSide::Left),
                            Some(si),
                            section,
                        )?;
                    }
                }
                // Right side services
                for (si, section) in wall_svcs.right.sections.iter().enumerate() {
                    if !section.is_empty() {
                        let label = format!("{} / Стена С{} (прав.)", room.name, wi + 1);
                        write_services(
                            sheet,
                            &mut est_row,
                            &mut grand_total,
                            &label,
                            *wall_id,
                            Some(WallSide::Right),
                            Some(si),
                            section,
                        )?;
                    }
                }
            }

            // Opening services for openings on this wall (windows only in room context)
            if let Some(wall) = project.wall(*wall_id) {
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
                    None,
                    svcs,
                )?;
            }
        }
    }

    // TOTAL row
    est_row += 1;
    write_str(sheet, est_row, 4, "ИТОГО:", &fmts.total)?;
    write_num(sheet, est_row, 5, grand_total, &fmts.total_currency)?;

    Ok(())
}
