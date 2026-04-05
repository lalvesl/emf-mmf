use crate::config::MotorConfig;
use crate::i18n::{Language, t};
use crate::phase;
use bevy_egui::egui;

pub fn mmf_ui(ui: &mut egui::Ui, config: &mut MotorConfig, lang: &Language) -> bool {
    let mut changed = false;

    if ui
        .checkbox(&mut config.mmf_field.show, t(lang, "show_mmf_field"))
        .on_hover_text(t(lang, "toggle_mmf_field_hover"))
        .changed()
    {
        changed = true;
        if config.mmf_field.show {
            for i in 0..config.phases {
                config.mmf_field.phases_to_show[i] = true;
            }
        }
    }

    if config.mmf_field.show {
        for i in 0..config.phases {
            let egui_color = phase::colors::phase_color_egui(i);
            let letter = phase::letter::phase_letter(i);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if ui
                    .checkbox(&mut config.mmf_field.phases_to_show[i], "")
                    .changed()
                {
                    changed = true;
                }

                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, egui_color);

                let phase_name = format!("{} {} ({})", t(&lang, "phase"), i + 1, letter);
                response.on_hover_text(&phase_name);

                ui.label(letter.to_string()).on_hover_text(&phase_name);
            });
        }
    }

    changed
}
