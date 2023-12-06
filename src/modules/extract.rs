use eframe::egui::{Button, ScrollArea, TextEdit, Ui};


#[derive(Default)]
pub struct Extract {
    multiup_links: String,
    recheck_hosts: bool,
    currently_extracting: bool,
}


impl Extract {
    pub fn display(ui: &mut Ui, extract: &mut Extract) {
        extract.display_input_area(ui);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {

        ui.heading("MultiUp Links");

        ui.vertical(|ui| {
            let input_area_height = ui.available_height() / 4.0;
            ui.set_max_height(input_area_height);
            ScrollArea::both()
                .id_source("MultiUp Link Input Area")
                .show(ui, |ui| {
                    ui.add(TextEdit::multiline(&mut self.multiup_links)
                        .hint_text("Paste your MultiUp links here")
                        .desired_width(ui.available_width()));
                });
        });

        // Recheck validity and extraction button
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.recheck_hosts, "Recheck link validity");
            if ui.add_enabled(!self.currently_extracting, Button::new("Extract links")).clicked() {
                todo!()
            }
        });
    }

    fn link_generation(&mut self, ui: &mut Ui) {

    }

    fn output_area(&mut self, ui: &mut Ui) {

    }

    fn filter_menu_area(&mut self, ui: &mut Ui) {

    }

}