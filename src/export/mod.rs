pub mod excel;
pub mod csv;
pub mod json;

use anyhow::Result;
use crate::models::PlcTable;

pub trait Exporter {
    fn export(&self, table: &PlcTable, path: &str) -> Result<()>;
}

pub fn export_to_clipboard(table: &PlcTable) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str("Address\tSymbol Name\tType\tComment\tPage\n");

    // Data rows
    for entry in &table.entries {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            entry.address,
            entry.symbol_name,
            entry.data_type,
            entry.comment,
            entry.page
        ));
    }

    Ok(output)
}