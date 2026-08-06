use crate::config::MotorConfig;
use crate::i18n::Strings;
use crate::ui::toggle_row;
use bevy_egui::egui;
use i18n::t;

pub fn winding_ui(ui: &mut egui::Ui, config: &mut MotorConfig) -> bool {
    toggle_row(
        ui,
        &mut config.show_endwindings,
        &t!(Strings::ShowHeaders),
        Some(&t!(Strings::ToggleHeadersHover)),
    )
}
