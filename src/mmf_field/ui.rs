use crate::config::{MAX_PHASES, MmfFieldConfig, MotorConfig, ViewConfig};
use crate::i18n::Strings;
use crate::phase;
use crate::ui::{float_slider, phase_swatch, slider_caption, toggle_row};
use bevy_egui::egui;
use egui_sc::egui_components::{Checkbox, Size, Spacing, Tooltip, small_text};
use i18n::t;

pub fn mmf_ui(ui: &mut egui::Ui, config: &MotorConfig, view: &mut ViewConfig) {
    // Never address more phases than `phases_to_show` can hold.
    let rows = config.phases.min(MAX_PHASES);

    // Switching the overlay on with nothing selected would show an empty
    // scene, so the first time it comes on every phase comes with it.
    if toggle_row(
        ui,
        &mut view.mmf_field.show,
        &t!(Strings::ShowMmfField),
        Some(&t!(Strings::ToggleMmfFieldHover)),
    ) && view.mmf_field.show
    {
        for shown in view.mmf_field.phases_to_show.iter_mut().take(rows) {
            *shown = true;
        }
    }

    if !view.mmf_field.show {
        return;
    }

    for i in 0..rows {
        let color = phase::colors::phase_color_egui(i, config.phases);
        let letter = phase::letter::phase_letter(i);
        let name = format!("{} {} ({})", t!(Strings::Phase), i + 1, letter);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            Checkbox::new(&mut view.mmf_field.phases_to_show[i])
                .size(Size::Sm)
                .show(ui);
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
        Checkbox::new(&mut view.mmf_field.show_result)
            .size(Size::Sm)
            .show(ui);
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
            view.mmf_field.gradient_intensity
        ),
    );

    Tooltip::new(&t!(Strings::MmfGradientIntensityHover)).wrap(ui, |ui| {
        float_slider(
            ui,
            "mmf_gradient",
            &mut view.mmf_field.gradient_intensity,
            MmfFieldConfig::MIN.gradient_intensity,
            MmfFieldConfig::MAX.gradient_intensity,
            0.1,
        )
    });
}
