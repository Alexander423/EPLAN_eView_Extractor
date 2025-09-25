use crate::models::{PlcEntry, PlcTable};
use egui_extras::{Column, TableBuilder};
use eframe::egui;

pub struct TableView {
    sort_column: SortColumn,
    sort_ascending: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum SortColumn {
    None,
    Address,
    Name,
    Type,
    Comment,
    Page,
}

impl TableView {
    pub fn new() -> Self {
        Self {
            sort_column: SortColumn::None,
            sort_ascending: true,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, table: &mut PlcTable, filter: &str) {
        // Header with table title and actions
        ui.horizontal(|ui| {
            ui.heading("SPS Table");
            ui.separator();

            let filtered_count = table.get_filtered(filter).len();
            let total_count = table.entries.len();

            if !filter.is_empty() {
                ui.label(format!("Showing {} of {} entries", filtered_count, total_count));
            } else {
                ui.label(format!("{} entries", total_count));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Select all/none buttons
                if ui.button("Select All").clicked() {
                    for entry in &mut table.entries {
                        if entry.matches_filter(filter) {
                            entry.selected = true;
                        }
                    }
                }

                if ui.button("Select None").clicked() {
                    table.select_all(false);
                }
            });
        });

        ui.separator();

        // The actual table
        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(40.0))  // Checkbox
            .column(Column::initial(100.0).at_least(80.0))  // Address
            .column(Column::initial(250.0).at_least(150.0)) // Symbol Name
            .column(Column::initial(80.0).at_least(60.0))   // Type
            .column(Column::remainder().at_least(200.0))    // Comment
            .column(Column::initial(80.0).at_least(60.0))   // Page
            .max_scroll_height(available_height)
            .header(25.0, |mut header| {
                // Checkbox header
                header.col(|ui| {
                    ui.strong("✓");
                });

                // Address header
                header.col(|ui| {
                    let response = ui.button("Address");
                    if response.clicked() {
                        self.toggle_sort(SortColumn::Address, table);
                    }
                    self.show_sort_indicator(ui, SortColumn::Address);
                });

                // Symbol Name header
                header.col(|ui| {
                    let response = ui.button("Symbol Name");
                    if response.clicked() {
                        self.toggle_sort(SortColumn::Name, table);
                    }
                    self.show_sort_indicator(ui, SortColumn::Name);
                });

                // Type header
                header.col(|ui| {
                    let response = ui.button("Type");
                    if response.clicked() {
                        self.toggle_sort(SortColumn::Type, table);
                    }
                    self.show_sort_indicator(ui, SortColumn::Type);
                });

                // Comment header
                header.col(|ui| {
                    let response = ui.button("Comment");
                    if response.clicked() {
                        self.toggle_sort(SortColumn::Comment, table);
                    }
                    self.show_sort_indicator(ui, SortColumn::Comment);
                });

                // Page header
                header.col(|ui| {
                    let response = ui.button("Page");
                    if response.clicked() {
                        self.toggle_sort(SortColumn::Page, table);
                    }
                    self.show_sort_indicator(ui, SortColumn::Page);
                });
            })
            .body(|mut body| {
                // Filter entries
                let entries: Vec<&mut PlcEntry> = table.entries
                    .iter_mut()
                    .filter(|entry| entry.matches_filter(filter))
                    .collect();

                for entry in entries {
                    let row_height = 22.0;
                    let data_type_color = entry.data_type.color();

                    body.row(row_height, |mut row| {
                        // Checkbox
                        row.col(|ui| {
                            ui.checkbox(&mut entry.selected, "");
                        });

                        // Address with color indicator
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                // Color indicator dot
                                let (response, painter) = ui.allocate_painter(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                painter.circle_filled(
                                    response.rect.center(),
                                    4.0,
                                    data_type_color,
                                );

                                ui.label(&entry.address);
                            });
                        });

                        // Symbol Name
                        row.col(|ui| {
                            ui.label(&entry.symbol_name);
                        });

                        // Type
                        row.col(|ui| {
                            ui.colored_label(data_type_color, entry.data_type.to_string());
                        });

                        // Comment (editable)
                        row.col(|ui| {
                            ui.text_edit_singleline(&mut entry.comment);
                        });

                        // Page
                        row.col(|ui| {
                            ui.label(&entry.page);
                        });
                    });
                }
            });
    }

    fn toggle_sort(&mut self, column: SortColumn, table: &mut PlcTable) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column.clone();
            self.sort_ascending = true;
        }

        self.apply_sort(table);
    }

    fn apply_sort(&self, table: &mut PlcTable) {
        match self.sort_column {
            SortColumn::Address => {
                if self.sort_ascending {
                    table.sort_by_address();
                } else {
                    table.sort_by_address();
                    table.entries.reverse();
                }
            }
            SortColumn::Name => {
                if self.sort_ascending {
                    table.sort_by_name();
                } else {
                    table.sort_by_name();
                    table.entries.reverse();
                }
            }
            SortColumn::Type => {
                if self.sort_ascending {
                    table.sort_by_type();
                } else {
                    table.sort_by_type();
                    table.entries.reverse();
                }
            }
            SortColumn::Comment => {
                table.entries.sort_by(|a, b| {
                    if self.sort_ascending {
                        a.comment.cmp(&b.comment)
                    } else {
                        b.comment.cmp(&a.comment)
                    }
                });
            }
            SortColumn::Page => {
                table.entries.sort_by(|a, b| {
                    if self.sort_ascending {
                        a.page.cmp(&b.page)
                    } else {
                        b.page.cmp(&a.page)
                    }
                });
            }
            SortColumn::None => {}
        }
    }

    fn show_sort_indicator(&self, ui: &mut egui::Ui, column: SortColumn) {
        if self.sort_column == column {
            let arrow = if self.sort_ascending { "▲" } else { "▼" };
            ui.label(arrow);
        }
    }
}