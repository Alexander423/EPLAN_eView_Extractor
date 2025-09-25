use serde::{Deserialize, Serialize};
use std::fmt;
use eframe::egui;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlcDataType {
    Input,
    Output,
    Memory,
    Unknown,
}

impl PlcDataType {
    pub fn from_address(address: &str) -> Self {
        if address.starts_with('I') {
            Self::Input
        } else if address.starts_with('Q') {
            Self::Output
        } else if address.starts_with('M') {
            Self::Memory
        } else {
            Self::Unknown
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            Self::Input => egui::Color32::from_rgb(46, 125, 50),   // Green
            Self::Output => egui::Color32::from_rgb(33, 150, 243), // Blue
            Self::Memory => egui::Color32::from_rgb(255, 193, 7),  // Amber
            Self::Unknown => egui::Color32::from_rgb(158, 158, 158), // Gray
        }
    }
}

impl fmt::Display for PlcDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input => write!(f, "Input"),
            Self::Output => write!(f, "Output"),
            Self::Memory => write!(f, "Memory"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlcEntry {
    pub address: String,
    pub symbol_name: String,
    pub data_type: PlcDataType,
    pub comment: String,
    pub page: String,
    pub selected: bool,
}

impl PlcEntry {
    pub fn new(address: String, symbol_name: String, page: String) -> Self {
        let data_type = PlcDataType::from_address(&address);
        Self {
            address,
            symbol_name,
            data_type,
            comment: String::new(),
            page,
            selected: false,
        }
    }

    pub fn matches_filter(&self, filter: &str) -> bool {
        if filter.is_empty() {
            return true;
        }

        let filter = filter.to_lowercase();
        self.address.to_lowercase().contains(&filter)
            || self.symbol_name.to_lowercase().contains(&filter)
            || self.comment.to_lowercase().contains(&filter)
            || self.page.to_lowercase().contains(&filter)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlcTable {
    pub entries: Vec<PlcEntry>,
    pub project_name: String,
    pub extraction_date: chrono::DateTime<chrono::Local>,
}

impl PlcTable {
    pub fn new(project_name: String) -> Self {
        Self {
            entries: Vec::new(),
            project_name,
            extraction_date: chrono::Local::now(),
        }
    }

    pub fn add_entry(&mut self, entry: PlcEntry) {
        self.entries.push(entry);
    }

    pub fn get_filtered(&self, filter: &str) -> Vec<&PlcEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.matches_filter(filter))
            .collect()
    }

    pub fn get_selected(&self) -> Vec<&PlcEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.selected)
            .collect()
    }

    pub fn select_all(&mut self, state: bool) {
        for entry in &mut self.entries {
            entry.selected = state;
        }
    }

    pub fn sort_by_address(&mut self) {
        self.entries.sort_by(|a, b| {
            natural_sort(&a.address, &b.address)
        });
    }

    pub fn sort_by_name(&mut self) {
        self.entries.sort_by(|a, b| {
            a.symbol_name.cmp(&b.symbol_name)
        });
    }

    pub fn sort_by_type(&mut self) {
        self.entries.sort_by(|a, b| {
            a.data_type.to_string().cmp(&b.data_type.to_string())
        });
    }
}

fn natural_sort(a: &str, b: &str) -> std::cmp::Ordering {
    // Extract numbers from addresses for natural sorting
    let extract_nums = |s: &str| -> (String, Vec<u32>) {
        let mut prefix = String::new();
        let mut numbers = Vec::new();
        let mut current_num = String::new();

        for ch in s.chars() {
            if ch.is_ascii_digit() {
                current_num.push(ch);
            } else {
                if !current_num.is_empty() {
                    if let Ok(num) = current_num.parse::<u32>() {
                        numbers.push(num);
                    }
                    current_num.clear();
                }
                if numbers.is_empty() {
                    prefix.push(ch);
                }
            }
        }

        if !current_num.is_empty() {
            if let Ok(num) = current_num.parse::<u32>() {
                numbers.push(num);
            }
        }

        (prefix, numbers)
    };

    let (prefix_a, nums_a) = extract_nums(a);
    let (prefix_b, nums_b) = extract_nums(b);

    match prefix_a.cmp(&prefix_b) {
        std::cmp::Ordering::Equal => nums_a.cmp(&nums_b),
        other => other,
    }
}