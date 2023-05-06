use eframe::egui;
use eframe::egui::{Context};

pub struct Help {}

impl Help {
    pub fn show(ctx: &Context, open: &mut bool) {
        egui::Window::new("Help").open(open).show(ctx, |ui| {
            ui.label("Instructions:\n\
            1. Paste your MultiUp links into the first box.\n\
            You can paste multiple links separated by a new line.\n\n\
            2. Choose whether you want MultiUp to check the validity of the hosts by checking \
            the \"Check host status\" checkbox.\n\
            3. Click \"Generate links\" to generate the direct links.\n\
            The application may freeze, but this is normal. \
            Please wait for the app to respond.\n\n\
            4. The direct links will be displayed in the second box.\n\
            You may copy them by right-clicking and selecting \"Copy links\" \
            or doing CTRL + A, CTRL + C.\n\n\
            5. You may also want to use the filter menu. \
            The \"Unknown\" checkbox includes links reported as unknown by MultiUp. \
            The \"Unchecked\" checkbox includes links that are unable to be checked. \
            You can also toggle which hosts you want links for.\n\n\
            Note:  Enabling “Check host status” will take longer to generate the links \
            compared to disabling the feature, but will give you more accurate information. \
            If you get unchecked links, it may be worth it to \
            generate the links with “Check host status” enabled.")
        });
    }
}
