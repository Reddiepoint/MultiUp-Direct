use eframe::egui;
use eframe::egui::{Context};

pub struct Help {}

impl Help {
    pub fn show(ctx: &Context, open: &mut bool) {
        egui::Window::new("Help").open(open).show(ctx, |ui| {
            ui.label("Instructions:\n\
            1. Paste your MultiUp links into the first box.\n\
            You can paste multiple links separated by a new line, \
            and it also supports both long and short links. \
            Duplicate links will automatically be removed.\n\n\
            2. Choose whether you want MultiUp to check the validity of the hosts by checking \
            the \"Check host status\" checkbox.\n\
            Enabling this feature will take a much longer time to generate the links \
            compared to disabling the feature, depending on the number of hosts the links have.\n\n\
            3. Click \"Generate links\" to generate the direct links.\n\
            The application may freeze, but this is normal. \
            If you have enabled \"Check host status\", this may last for a minute or so, \
            due to waiting for the hosts to be checked.\n\n\
            4. The direct links will be displayed in the second box.\n\
            You may copy them by right-clicking and selecting \"Copy links\" \
            or doing CTRL + A, CTRL + C. At this point, you may also decide to use the filter menu. \
            The \"Unknown\" checkbox includes links reported as unknown by MultiUp. \
            The \"Unchecked\" checkbox includes links that are unable to be checked. \
            You will only get unchecked links when you have \"Check host status\" disabled. \
            If you get unchecked links, \
            it may be worth it to generate the links with \"Check host status\" enabled.")
        });
    }
}
