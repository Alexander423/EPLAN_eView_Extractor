use regex::Regex;
use crate::models::{PlcEntry, PlcTable};

pub struct PlcDataExtractor;

impl PlcDataExtractor {
    pub fn parse_plc_data(input: &str) -> Vec<PlcEntry> {
        let mut results = Vec::new();

        // Split into lines
        let lines: Vec<&str> = input.lines().collect();

        // Regex patterns for parsing
        let address_pattern = Regex::new(r"\b([IQM]W?\d+\.\d+|[IQM]W\d+)\b").unwrap();
        let function_pattern = Regex::new(r"([A-Za-z][A-Za-z\s]+(?:\d+\.)+\d+(?:\s+[A-Z]+)?)").unwrap();

        let mut current_function = String::new();
        let mut current_page = String::new();

        for line in lines {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            // Skip header lines
            if Self::is_header_line(line) {
                continue;
            }

            // Check if this line contains page information
            if line.contains("Page") || line.contains("Sheet") {
                if let Some(page_num) = Self::extract_page_number(line) {
                    current_page = page_num;
                }
            }

            // Look for address
            if let Some(address_match) = address_pattern.find(line) {
                let address = address_match.as_str().to_string();

                // Extract function name before address
                let text_before = &line[..address_match.start()].trim();

                if let Some(func_match) = function_pattern.find(text_before) {
                    current_function = func_match.as_str().trim().to_string();
                } else if !text_before.is_empty() && !text_before.starts_with('=') {
                    // Use the text before address as function name
                    let parts: Vec<&str> = text_before.split_whitespace().collect();
                    let valid_parts: Vec<&str> = parts
                        .into_iter()
                        .filter(|p| !p.starts_with('=') && !p.starts_with(':'))
                        .collect();

                    if !valid_parts.is_empty() {
                        current_function = valid_parts.join(" ");
                    }
                }

                if !current_function.is_empty() {
                    let entry = PlcEntry::new(
                        address,
                        current_function.clone(),
                        current_page.clone(),
                    );
                    results.push(entry);
                }
            }
        }

        results
    }

    fn is_header_line(line: &str) -> bool {
        let skip_words = vec![
            "Sheet", "Editor", "Name", "GmbH", "Job", "Creator",
            "Version", "Approved", "IO-Test", "symbol name",
            "Function text", "Type:", "Placement:", "DT:",
            "Date", "Datum", "ET 200SP",
        ];

        skip_words.iter().any(|word| line.contains(word))
    }

    fn extract_page_number(line: &str) -> Option<String> {
        let page_pattern = Regex::new(r"(?:Page|Sheet)\s*[:=]?\s*(\S+)").unwrap();

        if let Some(captures) = page_pattern.captures(line) {
            if let Some(page_match) = captures.get(1) {
                return Some(page_match.as_str().to_string());
            }
        }

        None
    }

    pub fn extract_from_svg(svg_content: &str) -> Vec<String> {
        let mut extracted = Vec::new();

        // Pattern for text elements in SVG
        let text_pattern = Regex::new(r"<text[^>]*>([^<]+)</text>").unwrap();
        let tspan_pattern = Regex::new(r"<tspan[^>]*>([^<]+)</tspan>").unwrap();

        // Extract from text elements
        for cap in text_pattern.captures_iter(svg_content) {
            if let Some(text_match) = cap.get(1) {
                let text = text_match.as_str().trim();
                if !text.is_empty() && text.len() > 2 {
                    extracted.push(text.to_string());
                }
            }
        }

        // Extract from tspan elements
        for cap in tspan_pattern.captures_iter(svg_content) {
            if let Some(text_match) = cap.get(1) {
                let text = text_match.as_str().trim();
                if !text.is_empty() && text.len() > 2 {
                    extracted.push(text.to_string());
                }
            }
        }

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        let mut unique = Vec::new();

        for item in extracted {
            if seen.insert(item.clone()) {
                unique.push(item);
            }
        }

        unique
    }

    pub fn clean_and_format(entries: Vec<PlcEntry>) -> PlcTable {
        let mut table = PlcTable::new("Extracted Project".to_string());

        for entry in entries {
            table.add_entry(entry);
        }

        // Sort by address for better readability
        table.sort_by_address();

        table
    }
}