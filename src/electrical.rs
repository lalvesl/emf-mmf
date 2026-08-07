use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use egui_sc::egui_components::{
    Button, ButtonSize, ButtonVariant, ICON_EXPAND_MORE, ICON_PAUSE, ICON_PLAY_ARROW, ICON_WAVES,
    ShadcnTheme, Spacing, Tooltip, heading4, muted_text,
};
use i18n::t;

use crate::config::MotorConfig;
use crate::i18n::Strings;
use crate::theme::{axis_color, plot_corner};
use crate::ui::{PanelLayout, PanelSpace};

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
            .add_systems(
                EguiPrimaryContextPass,
                ui_electrical_waves
                    .in_set(PanelLayout::Bottom)
                    .run_if(crate::theme::fonts_ready),
            );
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
    mut space: ResMut<PanelSpace>,
    mut minimized: Local<bool>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // Only the space the side panels left over, so the two never overlap.
    let mut viewport_ui = space.ui(ctx, "electrical_waves_viewport");

    if *minimized {
        egui::Panel::bottom("electrical_minimized_panel")
            .resizable(false)
            .show(&mut viewport_ui, |ui| {
                if Button::new(&t!(Strings::ElectricalCurrents))
                    .icon(ICON_WAVES)
                    .variant(ButtonVariant::Ghost)
                    .show(ui)
                    .clicked()
                {
                    *minimized = false;
                }
            });
    } else {
        egui::Panel::bottom("electrical_currents_panel")
            .resizable(true)
            .show(&mut viewport_ui, |ui| {
                ui.horizontal(|ui| {
                    heading4(ui, &t!(Strings::ElectricalCurrents));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let clicked = Tooltip::new(&t!(Strings::MinimizePanelHover))
                            .wrap(ui, |ui| {
                                Button::new("")
                                    .icon(ICON_EXPAND_MORE)
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
                Spacing::Xs.show(ui);

                transport(ui, &mut state);
                Spacing::Sm.show(ui);

                draw_waveforms(ui, &config, &mut state);
            });
    }

    space.claim(&viewport_ui);
}

/// Play/pause and the speed slider.
fn transport(ui: &mut egui::Ui, state: &mut ElectricalState) {
    let (label, glyph) = if state.playing {
        (t!(Strings::Pause), ICON_PAUSE)
    } else {
        (t!(Strings::Play), ICON_PLAY_ARROW)
    };

    // Where the waveform is right now, which is what the playhead marks and
    // what every arrow in the scene is drawn from.
    let angle = state.angle.to_degrees().rem_euclid(360.0);

    ui.horizontal(|ui| {
        if Button::new(&label)
            .icon(glyph)
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Sm)
            .show(ui)
            .clicked()
        {
            state.playing = !state.playing;
        }

        muted_text(
            ui,
            &format!("{}: {:.2} Hz", t!(Strings::Speed), state.speed),
        );

        // Right to left, so the readout is pinned to the far end and the
        // slider takes whatever is left. Laid out the other way the label's
        // width would change with every degree and drag the slider with it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Tooltip::new(&t!(Strings::ElectricalAngle)).wrap(ui, |ui| {
                ui.scope(|ui| muted_text(ui, &format!("θ: {angle:3.0}°")))
                    .response
            });
            crate::ui::float_slider(ui, "speed", &mut state.speed, 0.05, 5.0, 0.05);
        });
    });
}

/// The per-phase current waveforms, with a scrubbable playhead.
fn draw_waveforms(ui: &mut egui::Ui, config: &MotorConfig, state: &mut ElectricalState) {
    let theme = ShadcnTheme::get(ui.ctx());

    // `Sense::drag()` alone never sets the click flag, so a plain click on the
    // plot could not seek — only a drag could.
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 150.0),
        egui::Sense::click_and_drag(),
    );

    let painter = ui.painter();
    painter.rect_filled(rect, plot_corner(&theme), theme.muted);

    // Axes
    let center_y = rect.center().y;
    painter.hline(rect.x_range(), center_y, (1.0, axis_color(&theme)));

    let m = config.phases;
    let width = rect.width();
    let height = rect.height() / 2.0;
    let alpha_m = crate::winding::axis::phase_displacement(m);

    for phase in 0..m {
        let color = crate::phase::colors::phase_color_egui(phase, m);

        let num_points = 100;
        let points: Vec<egui::Pos2> = (0..=num_points)
            .map(|i| {
                let t_val = i as f32 / num_points as f32;
                let angle = t_val * std::f32::consts::TAU;
                let y_normalized = crate::winding::axis::phase_current(angle, phase, alpha_m);
                egui::pos2(
                    rect.left() + t_val * width,
                    center_y - y_normalized * height * 0.9,
                )
            })
            .collect();

        painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, color)));
    }

    // Draggable bar logic
    if response.dragged() || response.clicked() {
        state.playing = false;
        if let Some(pos) = response.interact_pointer_pos() {
            let rel_x = (pos.x - rect.left()) / width;
            state.angle = rel_x.clamp(0.0, 1.0) * std::f32::consts::TAU;
        }
    }

    // Playhead
    let normalized_angle = state.angle.rem_euclid(std::f32::consts::TAU);
    let bar_x = rect.left() + (normalized_angle / std::f32::consts::TAU) * width;
    let painter = ui.painter();
    painter.vline(bar_x, rect.y_range(), (2.0, theme.foreground));
    painter.circle_filled(egui::pos2(bar_x, rect.top()), 4.0, theme.foreground);
}
