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
            .add_systems(EguiPrimaryContextPass, ui_panel);
    }
}

fn ui_panel(
    mut contexts: EguiContexts,
    mut config: ResMut<MotorConfig>,
    mut lang: ResMut<Language>,
    mut ev_writer: MessageWriter<MotorConfigChanged>,
    mut first_frame: Local<bool>,
    mut minimized: Local<bool>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut changed = false;

    // Trigger initial build
    if !*first_frame {
        changed = true;
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
        egui::SidePanel::left("motor_config_panel")
            .min_width(220.0)
            .default_width(260.0)
            .show(ctx, |ui| {
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
                if ui
                    .add(egui::Slider::new(
                        &mut grooves,
                        (MotorConfig::MIN.groove_count as i32)
                            ..=(MotorConfig::MAX.groove_count as i32),
                    ))
                    .changed()
                {
                    config.groove_count = grooves as usize;
                    clamp_config(&mut config);
                    changed = true;
                }
                ui.add_space(4.0);

                // Phases
                let mut phases = config.phases as i32;
                ui.label(format!("{} (m)", t(&lang, "phases")));
                if ui
                    .add(egui::Slider::new(
                        &mut phases,
                        (MotorConfig::MIN.phases as i32)..=(MotorConfig::MAX.phases as i32),
                    ))
                    .changed()
                {
                    config.phases = phases as usize;
                    clamp_config(&mut config);
                    changed = true;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for i in 0..config.phases {
                        let egui_color = phase::colors::phase_color_egui(i);
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
                if ui
                    .add(
                        egui::Slider::new(
                            &mut poles,
                            (MotorConfig::MIN.pole_pairs as i32 * 2)
                                ..=(MotorConfig::MAX.pole_pairs as i32 * 2),
                        )
                        .step_by(2.0),
                    )
                    .changed()
                {
                    config.pole_pairs = (poles / 2) as usize;
                    clamp_config(&mut config);
                    changed = true;
                }
                ui.add_space(4.0);

                // Layers
                let mut layers = config.layers as i32;
                ui.label(t(&lang, "layers"));
                if ui
                    .add(egui::Slider::new(
                        &mut layers,
                        (MotorConfig::MIN.layers as i32)..=(MotorConfig::MAX.layers as i32),
                    ))
                    .changed()
                {
                    config.layers = layers as usize;
                    changed = true;
                }
                ui.add_space(4.0);

                // Short-pitched
                if ui
                    .checkbox(&mut config.short_pitched, t(&lang, "short_pitched"))
                    .changed()
                {
                    changed = true;
                }
                if crate::winding::ui::winding_ui(ui, &mut config, &lang) {
                    changed = true;
                }
                if crate::mmf_field::ui::mmf_ui(ui, &mut config, &lang) {
                    changed = true;
                }
                if crate::rotor::ui::rotor_ui(ui, &mut config, &lang) {
                    changed = true;
                }
                if ui
                    .checkbox(&mut config.show_vectors, t(&lang, "show_vectors"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(&mut config.show_fields, t(&lang, "show_fields"))
                    .changed()
                {
                    changed = true;
                }

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
                    let alpha_str = format!("{} (α=P/2.360/S): {}°", t(&lang, "slot_angle"), alpha);

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
                        "{} {}: {}°",
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

    if changed {
        ev_writer.write(MotorConfigChanged);
    }
}

/// Ensure groove_count stays divisible by 2 * pole_pairs * phases.
fn clamp_config(config: &mut MotorConfig) {
    let divisor = 2 * config.pole_pairs * config.phases;
    if divisor > 0 && !config.groove_count.is_multiple_of(divisor) {
        // Snap to nearest valid value
        config.groove_count = ((config.groove_count + divisor / 2) / divisor) * divisor;
        config.groove_count = config.groove_count.clamp(
            divisor.max(MotorConfig::MIN.groove_count),
            MotorConfig::MAX.groove_count,
        );
    }
}
