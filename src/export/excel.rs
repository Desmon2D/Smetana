use std::path::Path;

use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook, Worksheet};

use crate::model::{PriceList, Project};
use super::excel_sheets::{write_rooms_sheet, write_doors_sheet, write_estimate_sheet};

/// Consolidated cell formats for Excel export.
pub(crate) struct ExcelFormats {
    pub header: Format,
    pub text: Format,
    pub number: Format,
    pub currency: Format,
    pub section: Format,
    pub total: Format,
    pub total_currency: Format,
}

impl ExcelFormats {
    pub fn new() -> Self {
        Self {
            header: Format::new()
                .set_bold()
                .set_align(FormatAlign::Center)
                .set_border(FormatBorder::Thin),
            text: Format::new().set_border(FormatBorder::Thin),
            number: Format::new()
                .set_num_format("0.00")
                .set_border(FormatBorder::Thin),
            currency: Format::new()
                .set_num_format("#,##0.00 \u{20bd}")
                .set_border(FormatBorder::Thin),
            section: Format::new()
                .set_bold()
                .set_font_size(12.0),
            total: Format::new()
                .set_bold()
                .set_border(FormatBorder::Thin),
            total_currency: Format::new()
                .set_bold()
                .set_num_format("#,##0.00 \u{20bd}")
                .set_border(FormatBorder::Thin),
        }
    }
}

/// Write a string cell with the given format, returning a uniform error.
pub(crate) fn write_str(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: &str,
    fmt: &Format,
) -> Result<(), String> {
    sheet
        .write_string_with_format(row, col, value, fmt)
        .map_err(|e| format!("Ошибка записи: {e}"))?;
    Ok(())
}

/// Write a number cell with the given format, returning a uniform error.
pub(crate) fn write_num(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: f64,
    fmt: &Format,
) -> Result<(), String> {
    sheet
        .write_number_with_format(row, col, value, fmt)
        .map_err(|e| format!("Ошибка записи: {e}"))?;
    Ok(())
}

/// Write a full header row using the header format.
pub(crate) fn write_header_row(
    sheet: &mut Worksheet,
    row: u32,
    headers: &[&str],
    fmt: &Format,
) -> Result<(), String> {
    for (col, header) in headers.iter().enumerate() {
        write_str(sheet, row, col as u16, header, fmt)?;
    }
    Ok(())
}

/// Export the project data to an Excel (.xlsx) file with three sheets:
/// "Помещения" (Rooms), "Двери" (Doors), "Смета" (Estimate).
pub fn export_to_xlsx(
    project: &Project,
    price_list: &PriceList,
    path: &Path,
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let fmts = ExcelFormats::new();

    // --- Sheet 1: Помещения (Rooms) ---
    let rooms_sheet = workbook.add_worksheet();
    rooms_sheet
        .set_name("Помещения")
        .map_err(|e| format!("Ошибка создания листа: {e}"))?;
    write_rooms_sheet(rooms_sheet, project, &fmts)?;

    // --- Sheet 2: Двери (Doors) ---
    let doors_sheet = workbook.add_worksheet();
    doors_sheet
        .set_name("Двери")
        .map_err(|e| format!("Ошибка создания листа: {e}"))?;
    write_doors_sheet(doors_sheet, project, &fmts)?;

    // --- Sheet 3: Смета (Estimate) ---
    let estimate_sheet = workbook.add_worksheet();
    estimate_sheet
        .set_name("Смета")
        .map_err(|e| format!("Ошибка создания листа: {e}"))?;
    write_estimate_sheet(estimate_sheet, project, price_list, &fmts)?;

    // Save the workbook
    workbook
        .save(path)
        .map_err(|e| format!("Ошибка сохранения файла: {e}"))?;

    Ok(())
}
