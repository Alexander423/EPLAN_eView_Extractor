use anyhow::Result;
use rust_xlsxwriter::Workbook;
use crate::models::{PlcTable, PlcDataType};
use super::Exporter;

pub struct ExcelExporter;

impl Exporter for ExcelExporter {
    fn export(&self, table: &PlcTable, path: &str) -> Result<()> {
        let mut workbook = Workbook::new();

        // Create worksheet
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("PLC Table")?;


        // Set column widths
        worksheet.set_column_width(0, 15)?;  // Address
        worksheet.set_column_width(1, 30)?;  // Symbol Name
        worksheet.set_column_width(2, 10)?;  // Type
        worksheet.set_column_width(3, 40)?;  // Comment
        worksheet.set_column_width(4, 10)?;  // Page

        // Write headers
        worksheet.write(0, 0, "Address")?;
        worksheet.write(0, 1, "Symbol Name")?;
        worksheet.write(0, 2, "Type")?;
        worksheet.write(0, 3, "Comment")?;
        worksheet.write(0, 4, "Page")?;

        // Freeze header row
        worksheet.set_freeze_panes(1, 0)?;

        // Enable autofilter
        worksheet.autofilter(0, 0, table.entries.len() as u32, 4)?;

        // Write data
        for (row_num, entry) in table.entries.iter().enumerate() {
            let row = (row_num + 1) as u32;

            // Write row data
            worksheet.write(row, 0, &entry.address)?;
            worksheet.write(row, 1, &entry.symbol_name)?;
            worksheet.write(row, 2, entry.data_type.to_string())?;
            worksheet.write(row, 3, &entry.comment)?;
            worksheet.write(row, 4, &entry.page)?;
        }

        // Create separate sheets for inputs and outputs
        self.create_filtered_sheet(&mut workbook, table, PlcDataType::Input, "Inputs")?;
        self.create_filtered_sheet(&mut workbook, table, PlcDataType::Output, "Outputs")?;

        // Add metadata sheet
        let meta_sheet = workbook.add_worksheet();
        meta_sheet.set_name("Metadata")?;
        meta_sheet.write(0, 0, "Project")?;
        meta_sheet.write(0, 1, &table.project_name)?;
        meta_sheet.write(1, 0, "Extraction Date")?;
        meta_sheet.write(1, 1, table.extraction_date.to_string())?;
        meta_sheet.write(2, 0, "Total Entries")?;
        meta_sheet.write(2, 1, table.entries.len() as f64)?;

        // Save workbook
        workbook.save(path)?;

        Ok(())
    }
}

impl ExcelExporter {
    fn create_filtered_sheet(
        &self,
        workbook: &mut Workbook,
        table: &PlcTable,
        filter_type: PlcDataType,
        sheet_name: &str,
    ) -> Result<()> {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(sheet_name)?;

        // Write headers
        worksheet.write(0, 0, "Address")?;
        worksheet.write(0, 1, "Symbol Name")?;
        worksheet.write(0, 2, "Comment")?;
        worksheet.write(0, 3, "Page")?;

        // Filter and write entries
        let filtered: Vec<_> = table.entries
            .iter()
            .filter(|e| e.data_type == filter_type)
            .collect();

        for (row_num, entry) in filtered.iter().enumerate() {
            let row = (row_num + 1) as u32;
            worksheet.write(row, 0, &entry.address)?;
            worksheet.write(row, 1, &entry.symbol_name)?;
            worksheet.write(row, 2, &entry.comment)?;
            worksheet.write(row, 3, &entry.page)?;
        }

        worksheet.autofilter(0, 0, filtered.len() as u32, 3)?;

        Ok(())
    }
}