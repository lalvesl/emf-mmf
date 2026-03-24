use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

use crate::config::{MotorConfig, MotorConfigChanged};
use crate::eletrical::ElectricalState;

pub struct FieldsPlugin;

impl Plugin for FieldsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (regenerate_fields, animate_fields));
    }
}

#[derive(Component)]
pub struct MagneticFieldText {
    pub phase: Option<usize>,
    pub pole: usize,
}

fn regenerate_fields(
    mut commands: Commands,
    mut ev_config: MessageReader<MotorConfigChanged>,
    config: Res<MotorConfig>,
    query: Query<Entity, With<MagneticFieldText>>,
) {
    if ev_config.read().next().is_none() {
        return;
    }

    // Despawn old field texts
    for entity in &query {
        commands.entity(entity).despawn();
    }

    if !config.show_fields {
        return;
    }

    let m = config.phases;
    let p = config.pole_pairs;
    if m == 0 || p == 0 {
        return;
    }

    // Spawn Phase Field Texts
    for pole in 0..(2 * p) {
        for phase in 0..m {
            let color = crate::colors::phase_color(phase);
            commands.spawn((
                Text2d::new(""),
                TextFont {
                    font_size: 160.0,
                    ..default()
                },
                TextColor(color),
                Transform::from_scale(Vec3::splat(0.01)),
                Visibility::default(),
                MagneticFieldText {
                    phase: Some(phase),
                    pole,
                },
            ));
        }

        // Spawn Resultant Field Text (White)
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font_size: 240.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_scale(Vec3::splat(0.01)),
            Visibility::default(),
            MagneticFieldText { phase: None, pole },
        ));
    }
}

fn animate_fields(
    config: Res<MotorConfig>,
    state: Res<ElectricalState>,
    mut query: Query<(
        &MagneticFieldText,
        &mut Transform,
        &mut Text2d,
        &mut Visibility,
    )>,
) {
    if !config.show_fields {
        return;
    }

    let m = config.phases;
    let p = config.pole_pairs;
    if m == 0 || p == 0 {
        return;
    }

    let elec_angle = state.angle;

    // Position text outside the stator iron radially
    let text_radius = crate::config::STATOR_OUTER_RADIUS * 1.15;

    let n = config.groove_count as f32;
    let m_f32 = m as f32;
    let p_f32 = p as f32;
    let q = n / (2.0 * p_f32 * m_f32);
    let pitch = crate::winding::coil_pitch(&config) as f32;

    let alpha = (p_f32 * TAU) / n;
    let alpha_m = if !config.phases.is_multiple_of(2) {
        TAU / m_f32
    } else {
        PI / m_f32
    };

    let offset_mech = (TAU / n) * 0.75;

    // We only need the amplitude and axis_phys for each phase/resultant
    let mut phase_amps: Vec<Vec<f32>> = vec![vec![0.0; m]; 2 * p];
    let mut phase_axes: Vec<Vec<f32>> = vec![vec![0.0; m]; 2 * p];

    let mut resultant_vecs: Vec<Vec3> = vec![Vec3::ZERO; 2 * p];
    let mut resultant_axes: Vec<f32> = vec![0.0; 2 * p];

    for pole in 0..(2 * p) {
        for phase in 0..m {
            let phase_shift_elec = phase as f32 * alpha_m;
            let current = (elec_angle - phase_shift_elec).cos();

            let start_elec = phase_shift_elec + (pole as f32 * PI);
            let offset_elec = (q - 1.0 + pitch) / 2.0 * alpha;
            let center_elec = start_elec + offset_elec;

            let axis_phys = (center_elec / p_f32) + offset_mech;
            let mmf_amplitude = current * if pole % 2 == 0 { 1.0 } else { -1.0 };

            phase_amps[pole][phase] = mmf_amplitude;
            phase_axes[pole][phase] = axis_phys;

            let dir = Vec3::new(axis_phys.cos(), 0.0, axis_phys.sin());
            resultant_vecs[pole] += dir * mmf_amplitude;
        }

        let res_len = resultant_vecs[pole].length();
        if res_len > 0.001 {
            resultant_axes[pole] = resultant_vecs[pole].z.atan2(resultant_vecs[pole].x);
        }
    }

    for (field_text, mut transform, mut text, mut visibility) in &mut query {
        let pole = field_text.pole;

        if pole >= 2 * p {
            continue;
        }

        let (amplitude, axis_phys) = if let Some(phase) = field_text.phase {
            if phase >= m {
                continue;
            }
            (phase_amps[pole][phase], phase_axes[pole][phase])
        } else {
            let res_vec = resultant_vecs[pole];
            let res_amp = res_vec.length();
            // The resultant vector points outward at `resultant_axes[pole]`.
            // Outward vector = North.
            (res_amp, resultant_axes[pole])
        };

        if amplitude.abs() > 0.05 {
            *visibility = Visibility::Inherited;

            let label = if amplitude >= 0.0 { "N" } else { "S" };
            text.0 = label.to_string();

            let x = text_radius * axis_phys.cos();
            let z = text_radius * axis_phys.sin();

            transform.translation = Vec3::new(x, 0.4, z); // Slightly elevated on Y to float above stator

            // Text reads correctly when viewed from outside
            transform.rotation = Quat::from_rotation_y(-axis_phys + PI / 2.0);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}
