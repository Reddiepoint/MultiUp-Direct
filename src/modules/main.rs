use eframe::{App, Frame};
use eframe::egui::{CentralPanel, Context, menu, TopBottomPanel, Ui};
use crate::modules::extract::Extract;


#[derive(Default)]
pub struct MultiUpDirect {
    tab_bar: TabBar,
    extract: Extract,
}

impl App for MultiUpDirect {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        MultiUpDirect::display_top_bar(self, ctx);
        MultiUpDirect::display_central_panel(self, ctx);
    }
}

impl MultiUpDirect {
    /// Displays the top bar.
    ///
    /// This method is responsible for rendering the top bar of the application, which includes
    /// the tab bar and the menu bar/toolbar elements.
    fn display_top_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("Tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Add tabs for each function
                ui.selectable_value(&mut self.tab_bar, TabBar::Extract, "Extract");

                // Menu bar/toolbar elements
                MultiUpDirect::menu_bar(ui);
            });
        });
    }

    /// Displays menu bar options in the top bar.
    ///
    /// This method is responsible for adding toolbar functionality for different options.
    fn menu_bar(ui: &mut Ui) {
        menu::bar(ui, |ui| {
            ui.menu_button("Help", |ui| {
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
            })
        });
    }


    /// Displays the central panel in the user interface based on the selected tab.
    ///
    /// The central panel must be added last.
    fn display_central_panel(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
           match &self.tab_bar {
               TabBar::Extract => Extract::display(ui, &mut self.extract)
           }
        });
    }
}


#[derive(Default, PartialEq)]
enum TabBar {
    #[default]
    Extract
}