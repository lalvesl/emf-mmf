use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};

use crate::config::MotorConfig;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_systems(Update, ui_panel);
    }
}

fn ui_panel(mut contexts: EguiContexts, mut config: ResMut<MotorConfig>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::SidePanel::left("motor_config_panel")
        .min_width(220.0)
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.heading("⚡ Motor Configuration");
            ui.separator();
            ui.add_space(8.0);

            // Groove count
            let mut grooves = config.groove_count as i32;
            ui.label("Grooves (slots)");
            if ui
                .add(egui::Slider::new(&mut grooves, 6..=72).step_by(1.0))
                .changed()
            {
                config.groove_count = grooves.max(6) as usize;
                clamp_config(&mut config);
            }
            ui.add_space(4.0);

            // Phases
            let mut phases = config.phases as i32;
            ui.label("Phases");
            if ui
                .add(egui::Slider::new(&mut phases, 1..=6).step_by(1.0))
                .changed()
            {
                config.phases = phases.max(1) as usize;
                clamp_config(&mut config);
            }
            ui.add_space(4.0);

            // Pole pairs
            let mut pole_pairs = config.pole_pairs as i32;
            ui.label("Pole pairs");
            if ui
                .add(egui::Slider::new(&mut pole_pairs, 1..=6).step_by(1.0))
                .changed()
            {
                config.pole_pairs = pole_pairs.max(1) as usize;
                clamp_config(&mut config);
            }
            ui.add_space(4.0);

            // Layers
            let mut layers = config.layers as i32;
            ui.label("Layers");
            if ui
                .add(egui::Slider::new(&mut layers, 1..=2).step_by(1.0))
                .changed()
            {
                config.layers = layers as usize;
            }
            ui.add_space(4.0);

            // Short-pitched
            ui.checkbox(&mut config.short_pitched, "Short-pitched coils");

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // Info panel
            let n = config.groove_count;
            let m = config.phases;
            let p = config.pole_pairs;
            let valid = m > 0 && p > 0 && n >= 2 * p * m && n % (2 * p * m) == 0;

            if valid {
                let q = n / (2 * p * m);
                let slots_per_pole = n / (2 * p);
                ui.label(format!("Slots per pole per phase: {}", q));
                ui.label(format!("Slots per pole: {}", slots_per_pole));
                ui.label(format!("Total poles: {}", 2 * p));
                ui.colored_label(egui::Color32::GREEN, "✓ Valid configuration");
            } else {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "⚠ Invalid: grooves must be divisible\n  by 2 × poles × phases",
                );
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label("🖱 Right-click drag to rotate");
            ui.label("🖱 Scroll to zoom");
        });
}

/// Ensure groove_count stays divisible by 2 * pole_pairs * phases.
fn clamp_config(config: &mut MotorConfig) {
    let divisor = 2 * config.pole_pairs * config.phases;
    if divisor > 0 && config.groove_count % divisor != 0 {
        // Snap to nearest valid value
        config.groove_count = ((config.groove_count + divisor / 2) / divisor) * divisor;
        config.groove_count = config.groove_count.max(divisor).min(72);
    }
}
