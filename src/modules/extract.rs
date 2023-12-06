use eframe::egui::{ScrollArea, TextEdit, Ui};


#[derive(Default)]
pub struct Extract {
    multiup_links: String
}


impl Extract {
    pub fn display(ui: &mut Ui, extract: &mut Extract) {
        extract.display_input_area(ui);
    }

    fn display_input_area(&mut self, ui: &mut Ui) {
        let input_area_height = ui.available_height() / 4.0;
        ui.heading("MultiUp Links");

        ui.vertical(|ui| {
            ui.set_max_height(input_area_height);
            ScrollArea::both()
                .id_source("MultiUp Link Input Area")
                .show(ui, |ui| {
                    ui.add(TextEdit::multiline(&mut self.multiup_links)
                        .hint_text("Paste your links here"));

                });
        });
    }

    fn output_area(&mut self, ui: &mut Ui) {

    }

    fn filter_menu_area(&mut self, ui: &mut Ui) {

    }

    fn link_generation(&mut self, ui: &mut Ui) {

    }
}