use crate::config::MotorConfig;
use crate::i18n::{Language, t};
use bevy_egui::egui;

pub fn winding_scheme_ui(ui: &mut egui::Ui, config: &mut MotorConfig, lang: &Language) -> bool {
    let mut changed = false;

    if ui
        .checkbox(&mut config.show_winding_scheme, t(lang, "show_winding_scheme"))
        .on_hover_text(t(lang, "show_winding_scheme_hover"))
        .changed()
    {
        changed = true;
    }

    changed
}
