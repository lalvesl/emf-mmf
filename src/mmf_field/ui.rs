use crate::config::MotorConfig;
use crate::i18n::{Language, t};
use bevy_egui::egui;

pub fn mmf_ui(ui: &mut egui::Ui, config: &mut MotorConfig, lang: &Language) -> bool {
    let changed = ui
        .checkbox(&mut config.mmf_field.show, t(lang, "show_mmf_field"))
        .on_hover_text(t(lang, "toggle_mmf_field_hover"))
        .changed();

    if config.mmf_field.show {
        for phase in 0..config.phases {
            ui.checkbox(
                &mut config.mmf_field.phases_to_show[phase],
                t(lang, &format!("phase_{}", phase)),
            );
        }
    }

    changed
}
