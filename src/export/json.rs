use anyhow::Result;
use serde_json;
use std::fs::File;
use std::io::Write;
use crate::models::PlcTable;
use super::Exporter;

pub struct JsonExporter {
    pretty: bool,
}

impl Default for JsonExporter {
    fn default() -> Self {
        Self { pretty: true }
    }
}

impl JsonExporter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }
}

impl Exporter for JsonExporter {
    fn export(&self, table: &PlcTable, path: &str) -> Result<()> {
        let json = if self.pretty {
            serde_json::to_string_pretty(table)?
        } else {
            serde_json::to_string(table)?
        };

        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;

        Ok(())
    }
}

pub fn export_for_tia_portal(table: &PlcTable) -> Result<String> {
    // Special format for future TIA Portal integration
    #[derive(serde::Serialize)]
    struct TiaTag {
        name: String,
        address: String,
        data_type: String,
        comment: String,
        retain: bool,
        accessible: bool,
        writable: bool,
    }

    let tia_tags: Vec<TiaTag> = table.entries
        .iter()
        .map(|entry| TiaTag {
            name: entry.symbol_name.clone(),
            address: entry.address.clone(),
            data_type: map_to_tia_type(&entry.address),
            comment: entry.comment.clone(),
            retain: false,
            accessible: true,
            writable: matches!(entry.data_type, crate::models::PlcDataType::Output),
        })
        .collect();

    Ok(serde_json::to_string_pretty(&tia_tags)?)
}

fn map_to_tia_type(address: &str) -> String {
    // Map EPLAN address format to TIA Portal data types
    if address.contains('.') {
        "Bool".to_string()
    } else if address.contains("W") {
        "Word".to_string()
    } else if address.contains("D") {
        "DWord".to_string()
    } else {
        "Bool".to_string()
    }
}