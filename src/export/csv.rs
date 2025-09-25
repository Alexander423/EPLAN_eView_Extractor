use anyhow::Result;
use csv::Writer;
use std::fs::File;
use crate::models::PlcTable;
use super::Exporter;

pub struct CsvExporter {
    delimiter: u8,
    with_bom: bool,
}

impl Default for CsvExporter {
    fn default() -> Self {
        Self {
            delimiter: b';',  // Semicolon for German Excel compatibility
            with_bom: true,   // UTF-8 BOM for Excel
        }
    }
}

impl CsvExporter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn with_bom(mut self, with_bom: bool) -> Self {
        self.with_bom = with_bom;
        self
    }
}

impl Exporter for CsvExporter {
    fn export(&self, table: &PlcTable, path: &str) -> Result<()> {
        let mut file = File::create(path)?;

        // Write BOM if requested (for Excel UTF-8 compatibility)
        if self.with_bom {
            use std::io::Write;
            file.write_all(&[0xEF, 0xBB, 0xBF])?;
        }

        let mut writer = Writer::from_writer(file);
        writer.write_record(&["Address", "Symbol Name", "Type", "Comment", "Page"])?;

        for entry in &table.entries {
            writer.write_record(&[
                &entry.address,
                &entry.symbol_name,
                &entry.data_type.to_string(),
                &entry.comment,
                &entry.page,
            ])?;
        }

        writer.flush()?;
        Ok(())
    }
}

pub fn export_multiple_csv(table: &PlcTable, prefix: &str) -> Result<()> {
    // Export all entries
    let all_exporter = CsvExporter::new();
    all_exporter.export(table, &format!("{}_all.csv", prefix))?;

    // Export inputs only
    let inputs_only = PlcTable {
        entries: table.entries
            .iter()
            .filter(|e| matches!(e.data_type, crate::models::PlcDataType::Input))
            .cloned()
            .collect(),
        project_name: table.project_name.clone(),
        extraction_date: table.extraction_date,
    };

    if !inputs_only.entries.is_empty() {
        all_exporter.export(&inputs_only, &format!("{}_inputs.csv", prefix))?;
    }

    // Export outputs only
    let outputs_only = PlcTable {
        entries: table.entries
            .iter()
            .filter(|e| matches!(e.data_type, crate::models::PlcDataType::Output))
            .cloned()
            .collect(),
        project_name: table.project_name.clone(),
        extraction_date: table.extraction_date,
    };

    if !outputs_only.entries.is_empty() {
        all_exporter.export(&outputs_only, &format!("{}_outputs.csv", prefix))?;
    }

    Ok(())
}