use crate::config::MotorConfig;
use crate::i18n::{Language, t};
use bevy_egui::egui;

pub fn winding_ui(ui: &mut egui::Ui, config: &mut MotorConfig, lang: &Language) -> bool {
    ui.checkbox(&mut config.show_endwindings, t(lang, "show_headers"))
        .on_hover_text(t(lang, "toggle_headers_hover"))
        .changed()
}
