use bevy::prelude::*;
use bevy_egui::egui;

const PHASE_SATURATION: f32 = 0.72;
const PHASE_LIGHTNESS: f32 = 0.50;

/// Phase color from chromatic circle division.
/// Hue evenly distributed across `total_phases`.
pub fn phase_color(phase: usize, total_phases: usize) -> Color {
    let total = total_phases.max(1);
    let hue = (phase % total) as f32 * 360.0 / total as f32;
    Color::from(bevy::color::Hsla::new(
        hue,
        PHASE_SATURATION,
        PHASE_LIGHTNESS,
        1.0,
    ))
}

pub fn phase_color_opposite(phase: usize, total_phases: usize) -> Color {
    let color = phase_color(phase, total_phases);
    let hsla: bevy::color::Hsla = color.into();
    Color::from(bevy::color::Hsla::new(
        (hsla.hue + 180.0) % 360.0,
        hsla.saturation,
        hsla.lightness,
        hsla.alpha,
    ))
}

pub fn phase_color_egui(phase: usize, total_phases: usize) -> egui::Color32 {
    let color: bevy::color::Srgba = phase_color(phase, total_phases).into();
    egui::Color32::from_rgb(
        (color.red * 255.0) as u8,
        (color.green * 255.0) as u8,
        (color.blue * 255.0) as u8,
    )
}
