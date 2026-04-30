use crate::config::MotorConfig;
use crate::i18n::{Language, t};
use bevy_egui::egui;

pub fn rotor_ui(ui: &mut egui::Ui, config: &mut MotorConfig, lang: &Language) -> bool {
    let mut changed = false;

    if ui
        .checkbox(&mut config.show_rotor, t(lang, "show_rotor"))
        .changed()
    {
        changed = true;
    }

    changed
}
