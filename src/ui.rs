use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use crate::config::{MotorConfig, MotorConfigChanged};
use crate::i18n::{Language, t};
use crate::phase;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_resource::<Language>()
            .init_resource::<PanelSpace>()
            .configure_sets(
                EguiPrimaryContextPass,
                (PanelLayout::Reset, PanelLayout::Side, PanelLayout::Bottom).chain(),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    reset_panel_space.in_set(PanelLayout::Reset),
                    ui_panel.in_set(PanelLayout::Side),
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
    /// Restores the full viewport as available space.
    Reset,
    /// Panels docked to a side edge.
    Side,
    /// Panels docked below whatever the side panels left.
    Bottom,
}

fn reset_panel_space(mut contexts: EguiContexts, mut space: ResMut<PanelSpace>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        space.0 = ctx.viewport_rect();
    }
}

fn ui_panel(
    mut contexts: EguiContexts,
    mut config: ResMut<MotorConfig>,
    mut lang: ResMut<Language>,
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
                if ui.button(t(&lang, "motor_config_btn")).clicked() {
                    *minimized = false;
                }
            });
    } else {
        egui::Panel::left("motor_config_panel")
            .min_size(220.0)
            .default_size(260.0)
            .show(&mut viewport_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(t(&lang, "motor_config_heading"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("⏴")
                            .on_hover_text(t(&lang, "minimize_panel_hover"))
                            .clicked()
                        {
                            *minimized = true;
                        }
                    });
                });

                // Language selector
                ui.horizontal(|ui| {
                    ui.label("🌐");
                    if ui.selectable_label(*lang == Language::PtBr, "PT").clicked() {
                        *lang = Language::PtBr;
                    }
                    if ui.selectable_label(*lang == Language::En, "EN").clicked() {
                        *lang = Language::En;
                    }
                });

                ui.separator();
                ui.add_space(8.0);

                // Groove count
                let mut grooves = config.groove_count as i32;
                ui.label(format!("{} (S)", t(&lang, "grooves")));
                let response = ui.add(egui::Slider::new(
                    &mut grooves,
                    (MotorConfig::MIN.groove_count as i32)
                        ..=(MotorConfig::MAX.groove_count as i32),
                ));
                if response.changed() {
                    config.groove_count = grooves as usize;
                    clamp_config(&mut config);
                }
                geometry_changed |= slider_settled(&response);
                ui.add_space(4.0);

                // Phases
                let mut phases = config.phases as i32;
                ui.label(format!("{} (m)", t(&lang, "phases")));
                let response = ui.add(egui::Slider::new(
                    &mut phases,
                    (MotorConfig::MIN.phases as i32)..=(MotorConfig::MAX.phases as i32),
                ));
                if response.changed() {
                    config.phases = phases as usize;
                    clamp_config(&mut config);
                }
                geometry_changed |= slider_settled(&response);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for i in 0..config.phases {
                        let egui_color = phase::colors::phase_color_egui(i, config.phases);
                        let letter = phase::letter::phase_letter(i);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let (rect, response) = ui
                                .allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, egui_color);

                            let phase_name =
                                format!("{} {} ({})", t(&lang, "phase"), i + 1, letter);
                            response.on_hover_text(&phase_name);

                            ui.label(letter.to_string()).on_hover_text(&phase_name);
                        });
                    }
                });
                ui.add_space(4.0);

                // Poles
                let mut poles = (config.pole_pairs * 2) as i32;
                ui.label(format!("{}(P)", t(&lang, "poles")));
                let response = ui.add(
                    egui::Slider::new(
                        &mut poles,
                        (MotorConfig::MIN.pole_pairs as i32 * 2)
                            ..=(MotorConfig::MAX.pole_pairs as i32 * 2),
                    )
                    .step_by(2.0),
                );
                if response.changed() {
                    config.pole_pairs = (poles / 2) as usize;
                    clamp_config(&mut config);
                }
                geometry_changed |= slider_settled(&response);
                ui.add_space(4.0);

                // Layers — conductors per slot, packed two per row
                let mut layers = config.layers as i32;
                let rows = config.layers.max(1).div_ceil(2);
                let cols = config.layers.clamp(1, 2);
                ui.label(format!("{} ({cols}×{rows})", t(&lang, "layers")));
                let response = ui.add(egui::Slider::new(
                    &mut layers,
                    (MotorConfig::MIN.layers as i32)..=(MotorConfig::MAX.layers as i32),
                ));
                if response.changed() {
                    config.layers = layers as usize;
                }
                geometry_changed |= slider_settled(&response);
                ui.add_space(4.0);

                // Short-pitched
                geometry_changed |= ui
                    .checkbox(&mut config.short_pitched, t(&lang, "short_pitched"))
                    .changed();

                // The remaining controls only change what is drawn, never the
                // shape of the machine.
                visibility_changed |= crate::winding::ui::winding_ui(ui, &mut config, &lang);
                visibility_changed |= crate::mmf_field::ui::mmf_ui(ui, &mut config, &lang);
                visibility_changed |= crate::rotor::ui::rotor_ui(ui, &mut config, &lang);
                visibility_changed |=
                    crate::winding_scheme::ui::winding_scheme_ui(ui, &mut config, &lang);
                visibility_changed |= ui
                    .checkbox(&mut config.show_vectors, t(&lang, "show_vectors"))
                    .changed();

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Info panel
                let n = config.groove_count;
                let m = config.phases;
                let p = config.pole_pairs;
                let valid = m > 0 && p > 0 && n >= 2 * p * m && n.is_multiple_of(2 * p * m);

                if valid {
                    let q = n / (2 * p * m);
                    let slots_per_pole = n / (2 * p);

                    let q_str = format!("{} (q=S/(m.P)): {}", t(&lang, "distribution_index"), q);
                    let spp_str = format!("{}: {}", t(&lang, "slots_per_pole"), slots_per_pole);
                    let poles_str = format!("{}: {}", t(&lang, "total_poles"), 2 * p);

                    let alpha = (p as f32 * 360.0) / (n as f32);
                    let alpha_str =
                        format!("{} (α=P/2.360/S): {:.2}°", t(&lang, "slot_angle"), alpha);

                    let alpha_m = if !m.is_multiple_of(2) {
                        360.0 / m as f32
                    } else {
                        180.0 / m as f32
                    };
                    let alpha_m_label = if !m.is_multiple_of(2) {
                        "(α.m=360/m)"
                    } else {
                        "(α.m=180/m)"
                    };
                    let alpha_m_str = format!(
                        "{} {}: {:.2}°",
                        t(&lang, "phase_angle"),
                        alpha_m_label,
                        alpha_m
                    );

                    ui.label(q_str);
                    ui.label(spp_str);
                    ui.label(poles_str);
                    ui.label(alpha_str);
                    ui.label(alpha_m_str);
                    ui.colored_label(egui::Color32::GREEN, t(&lang, "valid_config"));
                } else {
                    ui.colored_label(egui::Color32::YELLOW, t(&lang, "invalid_config"));
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(t(&lang, "rotate_hint"));
                ui.label(t(&lang, "zoom_hint"));
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

/// Whether a slider's value should be pushed to the scene this frame.
///
/// A regeneration rebuilds hundreds of meshes, and `changed()` fires on nearly
/// every frame of a drag, so a drag commits once on release. Clicks on the
/// track, arrow keys and typed values are not drags and commit immediately.
/// The value itself is always applied straight away, so the readouts in the
/// panel still follow the handle live.
fn slider_settled(response: &egui::Response) -> bool {
    response.drag_stopped() || (response.changed() && !response.dragged())
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
