use crate::config::ViewConfig;
use crate::i18n::Strings;
use crate::ui::toggle_row;
use bevy_egui::egui;
use i18n::t;

pub fn rotor_ui(ui: &mut egui::Ui, view: &mut ViewConfig) {
    toggle_row(ui, &mut view.show_rotor, &t!(Strings::ShowRotor), None);
}
