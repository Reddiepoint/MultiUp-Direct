use eframe::{App, Frame};
use eframe::egui::{CentralPanel, Context, menu, TopBottomPanel, Ui};
use crate::modules::extract::ExtractUI;
use crate::modules::help::HelpUI;


/// A struct representing the application UI.
/// 
/// Stores the state of each tab.
#[derive(Default)]
pub struct MultiUpDirect {
    tab_bar: TabBar,
    extract_ui: ExtractUI,
    help_ui: HelpUI,
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
                self.menu_bar(ui);

            });
        });
    }

    /// Displays menu bar options in the top bar.
    ///
    /// This method is responsible for adding toolbar functionality for different options.
    fn menu_bar(&mut self, ui: &mut Ui) {
        menu::bar(ui, |ui| {
            ui.menu_button("Help", |ui| {
                if ui.button("Show help").clicked() {
                    self.help_ui.show_help = true;
                    ui.close_menu();
                };


                ui.separator();

                if ui.button("Check for updates").clicked() {
                    let (tx, rx) = crossbeam_channel::unbounded();
                    // (self.help_ui.update_sender, self.help_ui.update_receiver) = (Some(tx), Some(rx));
                    self.help_ui.update_channels(tx, rx);
                    self.help_ui.show_update = true;
                    ui.close_menu();
                }


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
                TabBar::Extract => ExtractUI::display(ctx, ui, &mut self.extract_ui),
            }

            ExtractUI::display_error_log(&mut self.extract_ui, ctx);
            self.help_ui.show_help(ctx);
            self.help_ui.show_update(ctx);

        });
    }
}



/// Represents a bar containing tabs for each function.
#[derive(Default, PartialEq)]
enum TabBar {
    #[default]
    Extract,
}