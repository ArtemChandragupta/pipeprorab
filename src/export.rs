use crate::model::{CalculationResult, flatten_pipeline};
use rust_xlsxwriter::{Color, Format, Workbook, XlsxError};

pub fn export_to_excel(res: &CalculationResult, filename: &str) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Результаты")?;

    let header_format = Format::new().set_bold().set_background_color(Color::Silver);

    let headers = [
        "Элемент",
        "Расход (м³/с)",
        "P вх (кПа)",
        "P вых (кПа)",
        "dP (кПа)",
    ];

    for (col, &header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, header, &header_format)?;
    }

    let mut flat_tree = Vec::new();
    flatten_pipeline(&res.pipeline, 0, &mut flat_tree);

    for (row, (depth, comp)) in flat_tree.iter().enumerate() {
        let row_idx = (row + 1) as u32;

        let indent = "    ".repeat(*depth);
        let name_with_indent = format!("{}{}", indent, comp.name);

        worksheet.write_string(row_idx, 0, name_with_indent)?;
        worksheet.write_number(row_idx, 1, comp.state.q)?;
        worksheet.write_number(row_idx, 2, comp.state.p_in / 1000.0)?;
        worksheet.write_number(row_idx, 3, comp.state.p_out / 1000.0)?;

        let dp = (comp.state.p_in - comp.state.p_out) / 1000.0;
        worksheet.write_number(row_idx, 4, dp)?;
    }

    workbook.save(filename)?;
    Ok(())
}
