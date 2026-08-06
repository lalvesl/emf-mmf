use crate::config::{MAX_PHASES, MmfFieldConfig, MotorConfig, ViewConfig};
use crate::i18n::Strings;
use crate::phase;
use crate::ui::{color_chip, float_slider, slider_caption, toggle_row};
use bevy_egui::egui;
use egui_sc::egui_components::{Size, Spacing, Toggle, Tooltip};
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

    Spacing::Xs.show(ui);

    // One chip per phase, wrapping onto as many lines as the panel needs.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);

        for i in 0..rows {
            let letter = phase::letter::phase_letter(i);
            let name = format!("{} {} ({})", t!(Strings::Phase), i + 1, letter);
            phase_toggle(
                ui,
                &mut view.mmf_field.phases_to_show[i],
                phase::colors::phase_color_egui(i, config.phases),
                &letter.to_string(),
                &name,
            );
        }

        // White to match the resultant mesh, which carries no phase colour.
        let result = t!(Strings::MmfResult);
        phase_toggle(
            ui,
            &mut view.mmf_field.show_result,
            egui::Color32::WHITE,
            &result,
            &result,
        );
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

/// A chip carrying one series' colour and name, pressed while it is drawn.
fn phase_toggle(
    ui: &mut egui::Ui,
    shown: &mut bool,
    color: egui::Color32,
    label: &str,
    hover: &str,
) {
    let font = Size::Sm.font_size();
    Tooltip::new(hover).wrap(ui, |ui| {
        Toggle::custom(shown)
            .size(Size::Sm)
            .bordered(true)
            .show_with(ui, |ui| {
                // The surface itself cannot carry the phase colour — its fill
                // is what reports pressed or not — so the colour rides along
                // as a chip, the same one the phase legend uses.
                color_chip(ui, color);

                // A plain label, not `small_text`: the typography helpers pin
                // an explicit colour, and an explicit colour beats the
                // `override_text_color` that `show_with` sets from the toggle
                // state — the label would keep the muted tone once pressed.
                ui.label(egui::RichText::new(label).size(font));
            })
            .response
    });
}
