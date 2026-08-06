use crate::config::{MAX_PHASES, MmfFieldConfig, MotorConfig};
use crate::i18n::Strings;
use crate::phase;
use crate::ui::{phase_swatch, slider_caption, toggle_row};
use bevy_egui::egui;
use egui_sc::egui_components::{Checkbox, Size, Slider, Spacing, Tooltip, small_text};
use i18n::t;

pub fn mmf_ui(ui: &mut egui::Ui, config: &mut MotorConfig) -> bool {
    let mut changed = false;
    // Never address more phases than `phases_to_show` can hold.
    let rows = config.phases.min(MAX_PHASES);

    if toggle_row(
        ui,
        &mut config.mmf_field.show,
        &t!(Strings::ShowMmfField),
        Some(&t!(Strings::ToggleMmfFieldHover)),
    ) {
        changed = true;
        if config.mmf_field.show {
            for shown in config.mmf_field.phases_to_show.iter_mut().take(rows) {
                *shown = true;
            }
        }
    }

    if !config.mmf_field.show {
        return changed;
    }

    for i in 0..rows {
        let color = phase::colors::phase_color_egui(i, config.phases);
        let letter = phase::letter::phase_letter(i);
        let name = format!("{} {} ({})", t!(Strings::Phase), i + 1, letter);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            changed |= Checkbox::new(&mut config.mmf_field.phases_to_show[i])
                .size(Size::Sm)
                .show(ui)
                .clicked();
            phase_swatch(ui, color, &name);
            Tooltip::new(&name).wrap(ui, |ui| {
                ui.scope(|ui| small_text(ui, &letter.to_string())).response
            });
        });
    }

    Spacing::Xs.show(ui);

    // ── Result row ──────────────────────────────────────────────────────────
    let result_label = t!(Strings::MmfResult);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        changed |= Checkbox::new(&mut config.mmf_field.show_result)
            .size(Size::Sm)
            .show(ui)
            .clicked();
        // White swatch to match the result mesh colour.
        phase_swatch(ui, egui::Color32::WHITE, &result_label);
        Tooltip::new(&result_label).wrap(ui, |ui| {
            ui.scope(|ui| small_text(ui, &result_label)).response
        });
    });

    Spacing::Xs.show(ui);
    slider_caption(
        ui,
        &format!(
            "{}: {:.1}",
            t!(Strings::MmfGradientIntensity),
            config.mmf_field.gradient_intensity
        ),
    );

    let before = config.mmf_field.gradient_intensity;
    Tooltip::new(&t!(Strings::MmfGradientIntensityHover)).wrap(ui, |ui| {
        Slider::new(
            &mut config.mmf_field.gradient_intensity,
            MmfFieldConfig::MIN.gradient_intensity,
            MmfFieldConfig::MAX.gradient_intensity,
        )
        .step(0.1)
        .show(ui)
    });
    changed |= config.mmf_field.gradient_intensity != before;

    changed
}
