use std::path::Path;

use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};

use crate::model::{PriceList, Project};
use super::excel_sheets::{write_rooms_sheet, write_doors_sheet, write_estimate_sheet};

/// Export the project data to an Excel (.xlsx) file with three sheets:
/// "Помещения" (Rooms), "Двери" (Doors), "Смета" (Estimate).
pub fn export_to_xlsx(
    project: &Project,
    price_list: &PriceList,
    path: &Path,
) -> Result<(), String> {
    let mut workbook = Workbook::new();

    // --- Define cell formats ---
    let fmt_header = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);

    let fmt_text = Format::new().set_border(FormatBorder::Thin);

    let fmt_number = Format::new()
        .set_num_format("0.00")
        .set_border(FormatBorder::Thin);

    let fmt_currency = Format::new()
        .set_num_format("#,##0.00 ₽")
        .set_border(FormatBorder::Thin);

    let fmt_section = Format::new()
        .set_bold()
        .set_font_size(12.0);

    // --- Sheet 1: Помещения (Rooms) ---
    let rooms_sheet = workbook.add_worksheet();
    rooms_sheet
        .set_name("Помещения")
        .map_err(|e| format!("Ошибка создания листа: {e}"))?;
    write_rooms_sheet(rooms_sheet, project, &fmt_header, &fmt_text, &fmt_number, &fmt_section)?;

    // --- Sheet 2: Двери (Doors) ---
    let doors_sheet = workbook.add_worksheet();
    doors_sheet
        .set_name("Двери")
        .map_err(|e| format!("Ошибка создания листа: {e}"))?;
    write_doors_sheet(doors_sheet, project, &fmt_header, &fmt_text, &fmt_number)?;

    // --- Sheet 3: Смета (Estimate) ---
    let estimate_sheet = workbook.add_worksheet();
    estimate_sheet
        .set_name("Смета")
        .map_err(|e| format!("Ошибка создания листа: {e}"))?;
    write_estimate_sheet(estimate_sheet, project, price_list, &fmt_header, &fmt_text, &fmt_number, &fmt_currency)?;

    // Save the workbook
    workbook
        .save(path)
        .map_err(|e| format!("Ошибка сохранения файла: {e}"))?;

    Ok(())
}
