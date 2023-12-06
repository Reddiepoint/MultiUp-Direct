use eframe::{App, Frame};
use eframe::egui::{Context, menu, TopBottomPanel, Ui};
use crate::modules::extract::Extract;


#[derive(Default)]
pub struct Application {
    tab_bar: TabBar,
    extract: Extract,
}

impl App for Application {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        Application::top_bar(self, ctx);
    }
}

impl Application {
    /// Displays the top bar of the application.
    ///
    /// This method is responsible for rendering the top bar of the application, which includes
    /// the tab bar and the menu bar/toolbar elements.
    fn top_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("Tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Add tabs for each function
                ui.selectable_value(&mut self.tab_bar, TabBar::Extract, "Extract");

                // Menu bar/toolbar elements
                Application::menu_bar(ui);
            });
        });
    }

    fn menu_bar(ui: &mut Ui) {
        menu::bar(ui, |ui| {
            ui.menu_button("Help", |ui| {
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
            })
        });
    }

    fn show_central_panel(&mut self, ctx: &Context) {

    }
}


#[derive(Default, PartialEq)]
enum TabBar {
    #[default]
    Extract
}