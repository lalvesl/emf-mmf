use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use egui_sc::egui_components::{
    Alert, AlertVariant, Boxed, Button, ButtonGroup, ButtonGroupVariant, ButtonSize, ButtonVariant,
    Checkbox, ICON_CHECK_CIRCLE, ICON_CHEVRON_LEFT, ICON_LANGUAGE, ICON_MOUSE, ICON_TUNE,
    ICON_ZOOM_IN, Icon, Separator, ShadcnTheme, Size, Slider, Spacing, Switch, Tooltip, heading4,
    muted_text, small_text,
};
// Absolute path: `crate::i18n` below binds the name `i18n` in this module, and
// the macro lives in the crate of the same name.
use ::i18n::t;

use crate::config::{MotorConfig, MotorConfigChanged};
use crate::i18n::{self, Strings};
use crate::phase;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_plugins(i18n::I18nPlugin)
            .add_plugins(crate::theme::ThemePlugin)
            .init_resource::<PanelSpace>()
            .configure_sets(
                EguiPrimaryContextPass,
                (
                    PanelLayout::Theme,
                    PanelLayout::Reset,
                    PanelLayout::Side,
                    PanelLayout::Bottom,
                )
                    .chain(),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    reset_panel_space.in_set(PanelLayout::Reset),
                    ui_panel
                        .in_set(PanelLayout::Side)
                        .run_if(crate::theme::fonts_ready),
                ),
            );
    }
}

/// Screen space still free for docked panels this frame.
///
/// egui's `Panel` takes a `&mut Ui` and derives its area from that `Ui` alone,
/// so two panels built on two independently-created viewport `Ui`s both claim
/// the whole screen and overlap. This resource carries the leftover rect from
/// one panel system to the next; [`PanelLayout`] fixes the order they run in.
#[derive(Resource, Clone, Copy)]
pub struct PanelSpace(pub egui::Rect);

impl Default for PanelSpace {
    fn default() -> Self {
        Self(egui::Rect::NOTHING)
    }
}

impl PanelSpace {
    /// Build a `Ui` covering whatever space is still unclaimed.
    pub fn ui(&self, ctx: &egui::Context, id: impl egui::AsId) -> egui::Ui {
        egui::Ui::new(
            ctx.clone(),
            egui::Id::new(id),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(self.0),
        )
    }

    /// Hand what this panel left over to the panels that come after it.
    ///
    /// `Panel::show` advances the parent `Ui`'s cursor past the area it took,
    /// so the parent's remaining rect is exactly the leftover space.
    pub fn claim(&mut self, ui: &egui::Ui) {
        self.0 = ui.available_rect_before_wrap();
    }
}

/// Ordered stages for docked panels. Each stage claims from [`PanelSpace`], so
/// a later stage only ever sees what the earlier ones left behind.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelLayout {
    /// Publishes the Shadcn theme every component reads from.
    Theme,
    /// Restores the full viewport as available space.
    Reset,
    /// Panels docked to a side edge.
    Side,
    /// Panels docked below whatever the side panels left.
    Bottom,
}

// ─── Shared widget helpers ────────────────────────────────────────────────────

/// What a [`Slider`] did this frame.
///
/// The component senses drags only and never marks its `Response` as changed,
/// so both facts have to be derived here: `changed` by comparing the value, and
/// `settled` from the end of the drag.
pub struct SliderEdit {
    /// The value moved this frame — apply it so readouts follow the handle.
    pub changed: bool,
    /// The drag ended — safe to commit something expensive.
    pub settled: bool,
}

/// A slider over an integer parameter.
///
/// Regenerating the scene rebuilds hundreds of meshes, so a drag commits once
/// on release; the value itself is applied every frame regardless, which is
/// what keeps the readout in the label honest.
pub fn int_slider(
    ui: &mut egui::Ui,
    value: &mut usize,
    min: usize,
    max: usize,
    step: usize,
) -> SliderEdit {
    let mut raw = *value as f32;
    let response = Slider::new(&mut raw, min as f32, max as f32)
        .step(step.max(1) as f32)
        .show(ui);

    let stepped = (raw.round() as usize).clamp(min, max);
    let changed = stepped != *value;
    *value = stepped;

    SliderEdit {
        changed,
        settled: response.drag_stopped(),
    }
}

/// A labelled row that carries a value, above the control it belongs to.
///
/// The component slider paints no readout of its own, so the number lives here.
pub fn slider_caption(ui: &mut egui::Ui, text: &str) {
    muted_text(ui, text);
}

/// A phase's colour chip, with the phase name on hover.
pub fn phase_swatch(ui: &mut egui::Ui, color: egui::Color32, hover: &str) -> egui::Response {
    let radius = ShadcnTheme::get(ui.ctx()).radius * 0.5;
    Tooltip::new(hover).wrap(ui, |ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(radius as u8), color);
        response
    })
}

/// A visibility toggle: label, optional tooltip, and whether it flipped.
pub fn toggle_row(ui: &mut egui::Ui, value: &mut bool, label: &str, hover: Option<&str>) -> bool {
    match hover {
        Some(hover) => Tooltip::new(hover)
            .wrap(ui, |ui| Switch::new(value).label(label).show(ui))
            .clicked(),
        None => Switch::new(value).label(label).show(ui).clicked(),
    }
}

// ─── Winding factors (optional) ───────────────────────────────────────────────

/// Winding factors for the fundamental: what fraction of the ideal MMF this
/// winding actually produces, and where it went. `k_p` drops below 1 only when
/// the coils are chorded.
#[cfg(feature = "harmonics")]
fn winding_factor_labels(ui: &mut egui::Ui, config: &MotorConfig) {
    use crate::i18n::HarmonicStrings;
    use crate::winding::axis;

    let k_d = axis::distribution_factor(config, 1);
    let k_p = axis::pitch_factor(config, 1);

    let factor = |ui: &mut egui::Ui, text: String, hover: &str| {
        Tooltip::new(hover).wrap(ui, |ui| ui.scope(|ui| muted_text(ui, &text)).response);
    };

    factor(
        ui,
        format!(
            "{} (k_d): {:.4}",
            t!(HarmonicStrings::DistributionFactor),
            k_d.abs()
        ),
        &t!(HarmonicStrings::DistributionFactorHover),
    );
    factor(
        ui,
        format!(
            "{} (k_p): {:.4}",
            t!(HarmonicStrings::PitchFactor),
            k_p.abs()
        ),
        &t!(HarmonicStrings::PitchFactorHover),
    );

    let winding = format!(
        "{} (k_w=k_d.k_p): {:.4}",
        t!(HarmonicStrings::WindingFactor),
        (k_d * k_p).abs()
    );
    Tooltip::new(&t!(HarmonicStrings::WindingFactorHover))
        .wrap(ui, |ui| ui.scope(|ui| small_text(ui, &winding)).response);
}

#[cfg(not(feature = "harmonics"))]
fn winding_factor_labels(_: &mut egui::Ui, _: &MotorConfig) {}

// ─── Panel ────────────────────────────────────────────────────────────────────

fn reset_panel_space(mut contexts: EguiContexts, mut space: ResMut<PanelSpace>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        space.0 = ctx.viewport_rect();
    }
}

fn ui_panel(
    mut contexts: EguiContexts,
    mut config: ResMut<MotorConfig>,
    mut ev_writer: MessageWriter<MotorConfigChanged>,
    mut space: ResMut<PanelSpace>,
    mut first_frame: Local<bool>,
    mut minimized: Local<bool>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut viewport_ui = space.ui(ctx, "ui_panel_viewport");

    let mut geometry_changed = false;
    let mut visibility_changed = false;

    // Trigger initial build
    if !*first_frame {
        geometry_changed = true;
        *first_frame = true;
    }

    if *minimized {
        egui::Area::new(egui::Id::new("maximize_panel_area"))
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 10.0))
            .show(ctx, |ui| {
                if Button::new(&t!(Strings::MotorConfigBtn))
                    .icon(ICON_TUNE)
                    .variant(ButtonVariant::Secondary)
                    .show(ui)
                    .clicked()
                {
                    *minimized = false;
                }
            });
    } else {
        egui::Panel::left("motor_config_panel")
            .min_size(220.0)
            .default_size(280.0)
            .show(&mut viewport_ui, |ui| {
                ui.horizontal(|ui| {
                    heading4(ui, &t!(Strings::MotorConfigHeading));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let clicked = Tooltip::new(&t!(Strings::MinimizePanelHover))
                            .wrap(ui, |ui| {
                                Button::new("")
                                    .icon(ICON_CHEVRON_LEFT)
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Icon)
                                    .show(ui)
                            })
                            .clicked();
                        if clicked {
                            *minimized = true;
                        }
                    });
                });

                language_selector(ui);

                Spacing::Sm.show(ui);
                Separator::horizontal().show(ui);
                Spacing::Sm.show(ui);

                geometry_changed |= machine_controls(ui, &mut config);

                Spacing::Sm.show(ui);
                Separator::horizontal().show(ui);
                Spacing::Sm.show(ui);

                // The remaining controls only change what is drawn, never the
                // shape of the machine.
                visibility_changed |= crate::winding::ui::winding_ui(ui, &mut config);
                visibility_changed |= crate::mmf_field::ui::mmf_ui(ui, &mut config);
                visibility_changed |= crate::rotor::ui::rotor_ui(ui, &mut config);
                visibility_changed |= crate::winding_scheme::ui::winding_scheme_ui(ui, &mut config);
                visibility_changed |= toggle_row(
                    ui,
                    &mut config.show_vectors,
                    &t!(Strings::ShowVectors),
                    None,
                );

                Spacing::Md.show(ui);
                readout(ui, &config);

                Spacing::Md.show(ui);
                hints(ui);
            });
    }

    // When minimized the button is a floating `Area`, which reserves no space,
    // so the untouched `viewport_ui` correctly reports the whole viewport.
    space.claim(&viewport_ui);

    if geometry_changed {
        ev_writer.write(MotorConfigChanged::GEOMETRY);
    } else if visibility_changed {
        ev_writer.write(MotorConfigChanged::VISIBILITY);
    }
}

fn language_selector(ui: &mut egui::Ui) {
    let theme = ShadcnTheme::get(ui.ctx());
    let current = i18n::current_language();
    let tags: Vec<&str> = i18n::OFFERED.iter().copied().map(i18n::short_tag).collect();
    let selected = i18n::OFFERED.iter().position(|&lang| lang == current);

    ui.horizontal(|ui| {
        Icon::new(ICON_LANGUAGE)
            .size(16.0)
            .color(theme.muted_foreground)
            .show(ui)
            .on_hover_text(t!(Strings::Language));

        if let Some(picked) = ButtonGroup::new(&tags)
            .selected(selected)
            .variant(ButtonGroupVariant::Outline)
            .size(Size::Sm)
            .show(ui)
        {
            i18n::set_language(i18n::OFFERED[picked]);
        }
    });
}

/// The controls that change the shape of the machine. Returns whether anything
/// settled on a new value this frame.
fn machine_controls(ui: &mut egui::Ui, config: &mut MotorConfig) -> bool {
    let mut settled = false;

    // Groove count
    slider_caption(
        ui,
        &format!("{} (S): {}", t!(Strings::Grooves), config.groove_count),
    );
    let mut grooves = config.groove_count;
    let edit = int_slider(
        ui,
        &mut grooves,
        MotorConfig::MIN.groove_count,
        MotorConfig::MAX.groove_count,
        1,
    );
    if edit.changed {
        config.groove_count = grooves;
        clamp_config(config);
    }
    settled |= edit.settled;
    Spacing::Xs.show(ui);

    // Phases
    slider_caption(
        ui,
        &format!("{} (m): {}", t!(Strings::Phases), config.phases),
    );
    let mut phases = config.phases;
    let edit = int_slider(
        ui,
        &mut phases,
        MotorConfig::MIN.phases,
        MotorConfig::MAX.phases,
        1,
    );
    if edit.changed {
        config.phases = phases;
        clamp_config(config);
    }
    settled |= edit.settled;
    phase_legend(ui, config);
    Spacing::Xs.show(ui);

    // Poles — always even, so the slider steps in pole pairs.
    slider_caption(
        ui,
        &format!("{} (P): {}", t!(Strings::Poles), config.pole_pairs * 2),
    );
    let mut poles = config.pole_pairs * 2;
    let edit = int_slider(
        ui,
        &mut poles,
        MotorConfig::MIN.pole_pairs * 2,
        MotorConfig::MAX.pole_pairs * 2,
        2,
    );
    if edit.changed {
        config.pole_pairs = poles / 2;
        clamp_config(config);
    }
    settled |= edit.settled;
    Spacing::Xs.show(ui);

    // Layers — conductors per slot, packed two per row
    let packing = crate::winding::SlotPacking::new(config.layers);
    slider_caption(
        ui,
        &format!(
            "{} ({}×{}): {}",
            t!(Strings::Layers),
            packing.cols,
            packing.rows,
            config.layers
        ),
    );
    let mut layers = config.layers;
    let edit = int_slider(
        ui,
        &mut layers,
        MotorConfig::MIN.layers,
        MotorConfig::MAX.layers,
        1,
    );
    if edit.changed {
        config.layers = layers;
    }
    settled |= edit.settled;
    Spacing::Sm.show(ui);

    // Short-pitched — only meaningful with two electrical layers. The setting
    // is kept, not cleared, so toggling layers back on restores it; it simply
    // has no effect meanwhile.
    let can_chord = crate::winding::can_short_pitch(config);
    let label = t!(Strings::ShortPitched);
    let chord = |ui: &mut egui::Ui, config: &mut MotorConfig| {
        Checkbox::new(&mut config.short_pitched)
            .label(&label)
            .enabled(can_chord)
            .show(ui)
    };
    settled |= if can_chord {
        chord(ui, config).clicked()
    } else {
        // Disabled, so it can only be hovered — which is exactly when the
        // explanation is worth showing.
        let hint = t!(Strings::ShortPitchedNeedsLayers);
        Tooltip::new(&hint)
            .wrap(ui, |ui| chord(ui, config))
            .clicked()
    };

    settled
}

/// Colour chips for the phases of the current machine.
fn phase_legend(ui: &mut egui::Ui, config: &MotorConfig) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for i in 0..config.phases {
            let color = phase::colors::phase_color_egui(i, config.phases);
            let letter = phase::letter::phase_letter(i);
            let name = format!("{} {} ({})", t!(Strings::Phase), i + 1, letter);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                phase_swatch(ui, color, &name);
                Tooltip::new(&name).wrap(ui, |ui| {
                    ui.scope(|ui| small_text(ui, &letter.to_string())).response
                });
            });
        }
    });
}

/// Everything the panel reports rather than controls.
fn readout(ui: &mut egui::Ui, config: &MotorConfig) {
    let theme = ShadcnTheme::get(ui.ctx());

    let n = config.groove_count;
    let m = config.phases;
    let p = config.pole_pairs;
    let valid = m > 0 && p > 0 && n >= 2 * p * m && n.is_multiple_of(2 * p * m);

    if !valid {
        Alert::new(&t!(Strings::InvalidConfig))
            .description(&t!(Strings::InvalidConfigHint))
            .variant(AlertVariant::Warning)
            .show(ui);
        return;
    }

    let q = n / (2 * p * m);
    let slots_per_pole = n / (2 * p);
    let alpha = crate::winding::axis::slot_angle_elec(config).to_degrees();
    let alpha_m = crate::winding::axis::phase_displacement(m).to_degrees();
    let alpha_m_label = if m.is_multiple_of(2) {
        "(α.m=180/m)"
    } else {
        "(α.m=360/m)"
    };

    Boxed::new()
        .padding(Spacing::Sm)
        .accent(true)
        .show(ui, |ui| {
            muted_text(
                ui,
                &format!("{} (q=S/(m.P)): {q}", t!(Strings::DistributionIndex)),
            );
            muted_text(
                ui,
                &format!("{}: {slots_per_pole}", t!(Strings::SlotsPerPole)),
            );
            muted_text(ui, &format!("{}: {}", t!(Strings::TotalPoles), 2 * p));
            muted_text(
                ui,
                &format!("{} (α=P/2.360/S): {alpha:.2}°", t!(Strings::SlotAngle)),
            );
            muted_text(
                ui,
                &format!("{} {alpha_m_label}: {alpha_m:.2}°", t!(Strings::PhaseAngle)),
            );

            winding_factor_labels(ui, config);

            Spacing::Xs.show(ui);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                Icon::new(ICON_CHECK_CIRCLE)
                    .size(14.0)
                    .color(theme.success)
                    .show(ui);
                small_text(ui, &t!(Strings::ValidConfig));
            });
        });
}

/// Camera controls, as a footer.
fn hints(ui: &mut egui::Ui) {
    Separator::horizontal().show(ui);
    Spacing::Xs.show(ui);
    hint(ui, ICON_MOUSE, &t!(Strings::RotateHint));
    hint(ui, ICON_ZOOM_IN, &t!(Strings::ZoomHint));
}

/// An icon and a line of small print, on one row.
fn hint(ui: &mut egui::Ui, glyph: &'static str, text: &str) {
    let color = ShadcnTheme::get(ui.ctx()).muted_foreground;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        Icon::new(glyph).size(14.0).color(color).show(ui);
        small_text(ui, text);
    });
}

/// Ensure groove_count stays divisible by 2 * pole_pairs * phases.
///
/// Snapping happens *inside* the allowed range: the value is rounded to the
/// nearest multiple of `divisor` and then pulled back to the nearest multiple
/// that still fits `[MIN.groove_count, MAX.groove_count]`. Clamping the raw
/// rounded value instead would land on the bound itself, which is generally
/// not a multiple of `divisor`.
fn clamp_config(config: &mut MotorConfig) {
    let divisor = 2 * config.pole_pairs * config.phases;
    if divisor == 0 {
        return;
    }

    let min = MotorConfig::MIN.groove_count;
    let max = MotorConfig::MAX.groove_count;

    // Smallest and largest multiples of `divisor` inside the allowed range.
    let lowest = divisor * min.div_ceil(divisor);
    let highest = (max / divisor) * divisor;

    if lowest > highest {
        // No slot count in range satisfies the divisibility rule; leave the
        // value alone so the panel reports the configuration as invalid.
        return;
    }

    let snapped = ((config.groove_count + divisor / 2) / divisor) * divisor;
    config.groove_count = snapped.clamp(lowest, highest);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MAX_PHASES, MmfFieldConfig};

    /// Every value reachable through the sliders must come out of
    /// `clamp_config` satisfying the divisibility rule and staying in range.
    #[test]
    fn clamp_config_always_yields_a_valid_configuration() {
        let min = MotorConfig::MIN.groove_count;
        let max = MotorConfig::MAX.groove_count;

        for pole_pairs in MotorConfig::MIN.pole_pairs..=MotorConfig::MAX.pole_pairs {
            for phases in MotorConfig::MIN.phases..=MotorConfig::MAX.phases {
                for groove_count in min..=max {
                    let mut config = MotorConfig {
                        groove_count,
                        phases,
                        pole_pairs,
                        ..default()
                    };
                    clamp_config(&mut config);

                    let divisor = 2 * pole_pairs * phases;
                    assert!(
                        config.groove_count.is_multiple_of(divisor),
                        "S={} not divisible by 2*p*m={} (p={}, m={}, from S={})",
                        config.groove_count,
                        divisor,
                        pole_pairs,
                        phases,
                        groove_count,
                    );
                    assert!(
                        (min..=max).contains(&config.groove_count),
                        "S={} out of range (p={}, m={}, from S={})",
                        config.groove_count,
                        pole_pairs,
                        phases,
                        groove_count,
                    );
                }
            }
        }
    }

    /// Regression: rounding up used to overshoot `MAX.groove_count`, and the
    /// clamp then landed on 144 — not a multiple of 96 — leaving the panel
    /// stuck on "invalid configuration" with an empty scene.
    #[test]
    fn clamp_config_snaps_down_instead_of_clamping_to_the_bound() {
        let mut config = MotorConfig {
            groove_count: 144,
            phases: 8,
            pole_pairs: 6,
            ..default()
        };
        clamp_config(&mut config);
        assert_eq!(config.groove_count, 96);
    }

    /// The phase sliders must never be able to address past the end of
    /// `MmfFieldConfig::phases_to_show`.
    #[test]
    fn every_selectable_phase_is_addressable_in_the_mmf_field() {
        assert_eq!(MotorConfig::MAX.phases, MAX_PHASES);
        assert_eq!(MmfFieldConfig::default().phases_to_show.len(), MAX_PHASES);

        let field = MmfFieldConfig::MAX;
        for phase in 0..MotorConfig::MAX.phases {
            assert!(field.shows_phase(phase), "phase {phase} not addressable");
        }
        // Stale indices report hidden rather than panicking.
        assert!(!field.shows_phase(MAX_PHASES));
    }
}
