use eframe::egui::Ui;

pub struct Settings {}

impl Settings {
	pub fn show(ui: &mut Ui) {
		ui.label("Nothing here yet :)");
		ui.label("Made by Redpoint\nSorry for making such a bad application");
	}
}