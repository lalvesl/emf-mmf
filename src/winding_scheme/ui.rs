use crate::config::ViewConfig;
use crate::i18n::Strings;
use crate::ui::toggle_row;
use bevy_egui::egui;
use i18n::t;

pub fn winding_scheme_ui(ui: &mut egui::Ui, view: &mut ViewConfig) {
    toggle_row(
        ui,
        &mut view.show_winding_scheme,
        &t!(Strings::ShowWindingScheme),
        Some(&t!(Strings::ShowWindingSchemeHover)),
    );
}
