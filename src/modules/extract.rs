use eframe::egui::{Align, Button, Layout, ScrollArea, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};


#[derive(Default)]
pub struct ExtractUI {
    multiup_links: String,
    recheck_validity: bool,
    currently_extracting: bool,
}


impl ExtractUI {
    pub fn display(ui: &mut Ui, extract_ui: &mut ExtractUI) {
        extract_ui.display_input_area(ui);
        extract_ui.display_link_information(ui);
        extract_ui.display_output_area(ui);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        ui.heading("MultiUp Links");

        let input_area_height = ui.available_height() / 4.0;
        ui.set_max_height(input_area_height);
        ScrollArea::both()
            .id_source("MultiUp Link Input Area")
            .max_height(input_area_height)
            .show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut self.multiup_links)
                    .hint_text("Paste your MultiUp links here")
                    .desired_width(ui.available_width()));
            });

        // Recheck validity and extraction button
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.recheck_validity, "Recheck link validity");

            if ui.add_enabled(!self.currently_extracting, Button::new("Extract links")).clicked() {
                extract_links();
            }
        });
    }

    fn display_link_information(&mut self, ui: &mut Ui) {
        ui.collapsing("Link Information", |ui| {
            let width = ui.available_width();
            TableBuilder::new(ui)
                // Column for selecting which MultiUp links to show
                .column(Column::auto())
                // Column for MultiUp link information
                .column(Column::remainder())
                .cell_layout(Layout::left_to_right(Align::Center))
                .body(|body| {

                });
        });
    }

    fn display_output_area(&mut self, ui: &mut Ui) {
        ui.heading("Direct Links");

        ui.horizontal(|ui| {
            let output_box_width = 0.75 * ui.available_width();
            TableBuilder::new(ui)
                .column(Column::exact(output_box_width))
                .body(|body| {

                });

            self.display_filter_menu_area(ui);
        });
    }

    fn display_filter_menu_area(&mut self, ui: &mut Ui) {

    }
}

fn extract_links() {

}