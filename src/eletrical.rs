use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::config::MotorConfig;
use crate::i18n::{Language, t};

pub struct EletricalPlugin;

#[derive(Resource)]
pub struct ElectricalState {
    pub angle: f32, // Electrical angle in radians
    pub playing: bool,
    pub speed: f32, // Hz (electrical cycles per second)
}

impl Default for ElectricalState {
    fn default() -> Self {
        Self {
            angle: 0.0,
            playing: true,
            speed: 1.0,
        }
    }
}

impl Plugin for EletricalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ElectricalState>()
            .add_systems(Update, update_electrical_angle)
            .add_systems(EguiPrimaryContextPass, ui_electrical_waves);
    }
}

fn update_electrical_angle(time: Res<Time>, mut state: ResMut<ElectricalState>) {
    if state.playing {
        // radians = cycles * TAU
        state.angle += state.speed * std::f32::consts::TAU * time.delta_secs();
        if state.angle > std::f32::consts::TAU {
            state.angle %= std::f32::consts::TAU;
        }
    }
}

fn ui_electrical_waves(
    mut contexts: EguiContexts,
    mut state: ResMut<ElectricalState>,
    config: Res<MotorConfig>,
    lang: Res<Language>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::TopBottomPanel::bottom("electrical_currents_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading(t(&lang, "electrical_currents"));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let play_text = if state.playing {
                    t(&lang, "pause")
                } else {
                    t(&lang, "play")
                };
                if ui.button(play_text).clicked() {
                    state.playing = !state.playing;
                }
                ui.add(
                    egui::Slider::new(&mut state.speed, 0.05..=5.0)
                        .step_by(0.05)
                        .text(t(&lang, "speed")),
                );
            });

            ui.add_space(10.0);

            // Draw the waves
            let (rect, response) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 150.0), egui::Sense::drag());

            // Background
            ui.painter()
                .rect_filled(rect, 4.0, egui::Color32::from_black_alpha(50));

            // Axes
            let center_y = rect.center().y;
            ui.painter().hline(
                rect.x_range(),
                center_y,
                (1.0, egui::Color32::from_gray(100)),
            );

            let m = config.phases;
            let width = rect.width();
            let height = rect.height() / 2.0;

            for phase in 0..m {
                let color_bevy = crate::colors::phase_color(phase);
                let srgba: bevy::color::Srgba = color_bevy.into();
                let color_egui = egui::Color32::from_rgba_unmultiplied(
                    (srgba.red * 255.0) as u8,
                    (srgba.green * 255.0) as u8,
                    (srgba.blue * 255.0) as u8,
                    255,
                );

                let phase_shift = if m % 2 != 0 {
                    phase as f32 * 360.0 / (m as f32) // degrees
                } else {
                    phase as f32 * 180.0 / (m as f32) // degrees
                }
                .to_radians();

                let mut points = vec![];
                let num_points = 100;
                for i in 0..=num_points {
                    let t_val = i as f32 / num_points as f32;
                    let x = rect.left() + t_val * width;
                    let angle = t_val * std::f32::consts::TAU;

                    // I_phase = cos(angle - phase_shift)
                    let y_normalized = (angle - phase_shift).cos();
                    let y = center_y - y_normalized * height * 0.9;

                    points.push(egui::pos2(x, y));
                }

                ui.painter().add(egui::Shape::line(
                    points,
                    egui::Stroke::new(2.0, color_egui),
                ));
            }

            // Draggable bar logic
            if response.dragged() || response.clicked() {
                state.playing = false;
                if let Some(pos) = response.interact_pointer_pos() {
                    let rel_x = (pos.x - rect.left()) / width;
                    state.angle = rel_x.clamp(0.0, 1.0) * std::f32::consts::TAU;
                }
            }

            // Draw current state bar
            let normalized_angle = state.angle.rem_euclid(std::f32::consts::TAU);
            let bar_x = rect.left() + (normalized_angle / std::f32::consts::TAU) * width;
            ui.painter()
                .vline(bar_x, rect.y_range(), (2.0, egui::Color32::WHITE));

            // Draw a small handle on top of the bar
            ui.painter()
                .circle_filled(egui::pos2(bar_x, rect.top()), 4.0, egui::Color32::WHITE);
        });
}
