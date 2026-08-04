use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use std::f32::consts::TAU;

use crate::config::MotorConfig;
use crate::electrical::ElectricalState;
use crate::i18n::{self, Language};
use crate::phase;
use crate::winding::{Direction, compute_conductors, compute_winding};

pub struct WindingSchemePlugin;

impl Plugin for WindingSchemePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, winding_scheme_window);
    }
}

/// How many angular samples to use when drawing the MMF step waveforms.
const WAVEFORM_SAMPLES: usize = 720;

fn winding_scheme_window(
    mut contexts: EguiContexts,
    config: Res<MotorConfig>,
    state: Res<ElectricalState>,
    lang: Res<Language>,
) {
    if !config.show_winding_scheme {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new(i18n::t(&lang, "winding_scheme_title"))
        .id(egui::Id::new("winding_scheme_window"))
        .default_width(620.0)
        .min_width(400.0)
        .resizable(true)
        .show(ctx, |ui| {
            let assignments = compute_winding(&config);
            let conductors = compute_conductors(&config, &assignments);

            // ── Top panel: conductor layout ─────────────────────────────────
            // Height grows with the stack so a 6-conductor slot still fits.
            let rows = config.layers.max(1).div_ceil(2);
            let conductor_panel_height = 46.0 + rows as f32 * 22.0;
            let (rect_top, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), conductor_panel_height),
                egui::Sense::hover(),
            );
            draw_conductor_panel(ui, rect_top, &config, &conductors, &lang);

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // ── Bottom panel: MMF waveforms ─────────────────────────────────
            ui.label(egui::RichText::new(i18n::t(&lang, "winding_function_mmf")).strong());
            let waveform_panel_height = 180.0_f32;
            let (rect_bot, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), waveform_panel_height),
                egui::Sense::hover(),
            );
            draw_mmf_panel(ui, rect_bot, &config, &conductors, state.angle, &lang);
        });
}

fn draw_conductor_panel(
    ui: &egui::Ui,
    rect: egui::Rect,
    config: &MotorConfig,
    conductors: &[crate::winding::Conductor],
    _lang: &Language,
) {
    let painter = ui.painter_at(rect);
    let n = config.groove_count;

    let cy = rect.bottom() - 8.0;
    // Spread the n slots over the full rect width with some padding
    let padding = 24.0;
    let slot_step = (rect.width() - padding * 2.0) / n as f32;

    // Mirror the 3D packing: two conductors per row, deepest row first.
    let count = config.layers.max(1);
    let cols = count.min(2);
    let rows = count.div_ceil(cols);

    let row_step = 20.0_f32;
    let sym_r = (slot_step * 0.38).min(row_step * 0.42);
    let col_step = sym_r * 1.05;

    // Tick marks, one per slot
    for s in 0..n {
        let x = rect.left() + padding + (s as f32 + 0.5) * slot_step;
        painter.line_segment(
            [egui::pos2(x, cy), egui::pos2(x, cy - 5.0)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)),
        );
    }

    // Draw conductor symbols, stacked so both electrical layers are visible
    for conductor in conductors {
        let slot_x = rect.left() + padding + (conductor.slot as f32 + 0.5) * slot_step;

        let row = conductor.index / cols;
        let col = conductor.index % cols;
        let in_row = if row == rows - 1 {
            count - row * cols
        } else {
            cols
        };

        // Row 0 is the slot bottom, drawn at the top of the stack.
        let x = slot_x + (col as f32 - (in_row as f32 - 1.0) / 2.0) * col_step;
        let y = cy - 12.0 - (rows - 1 - row) as f32 * row_step;

        let base_color = phase::colors::phase_color_egui(conductor.phase, config.phases);

        // Circle outline
        painter.circle_stroke(egui::pos2(x, y), sym_r, egui::Stroke::new(1.5, base_color));

        match conductor.direction {
            // Current coming OUT of the page: ⊕ (dot)
            Direction::Out => {
                painter.circle_filled(egui::pos2(x, y), sym_r * 0.32, base_color);
            }
            // Current going INTO the page: ⊗ (cross)
            Direction::In => {
                let d = sym_r * 0.55;
                let col = base_color;
                painter.line_segment(
                    [egui::pos2(x - d, y - d), egui::pos2(x + d, y + d)],
                    egui::Stroke::new(1.5, col),
                );
                painter.line_segment(
                    [egui::pos2(x + d, y - d), egui::pos2(x - d, y + d)],
                    egui::Stroke::new(1.5, col),
                );
            }
        }
    }

    // Legend: phase symbols
    {
        let mut lx = rect.left() + padding;
        let ly = rect.top() + 12.0;
        for ph in 0..config.phases {
            let col = phase::colors::phase_color_egui(ph, config.phases);
            let letter = crate::phase::letter::phase_letter(ph);

            // small circle
            painter.circle_stroke(egui::pos2(lx + 6.0, ly), 5.0, egui::Stroke::new(1.5, col));
            painter.circle_filled(egui::pos2(lx + 14.0 + 2.0, ly), 1.8, col);
            painter.text(
                egui::pos2(lx + 26.0, ly),
                egui::Align2::LEFT_CENTER,
                letter.to_string(),
                egui::FontId::proportional(10.0),
                col,
            );
            lx += 36.0;
        }
    }
}

// ─── MMF waveform panel (bottom panel) ────────────────────────────────────────

fn draw_mmf_panel(
    ui: &egui::Ui,
    rect: egui::Rect,
    config: &MotorConfig,
    conductors: &[crate::winding::Conductor],
    elec_angle: f32,
    lang: &Language,
) {
    let painter = ui.painter_at(rect);
    let n = config.groove_count;
    let m = config.phases;

    // Background
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 28));

    // ── Compute winding-function / MMF waveforms ─────────────────────────────
    // The winding function N_k(θ) for phase k is the sum of all conductors weighted by
    // +1 (Out) or -1 (In) that are "active" from position 0 to θ.
    //
    // For a display purposes, we compute the expected (time-averaged) winding function
    // per phase. Each slot is at mechanical angle θ_s = 2π * s / n.
    // Then MMF_k(θ) = N_k(θ) * i_k(t).
    //
    // We build a piecewise-constant winding function by accumulating conductor
    // contributions at each sample point.

    // Winding functions: wf[phase][sample] — computed over [0, 2π]
    let mut wf: Vec<Vec<f32>> = vec![vec![0.0; WAVEFORM_SAMPLES]; m.max(1)];

    // Each conductor contributes a unit step at θ_s, so record the step at the
    // first sample with θ ≥ θ_s and integrate once per phase afterwards. Doing
    // it as a running sum keeps this O(n + m·samples) instead of O(n·samples),
    // which matters because the window redraws every frame.
    // Every conductor counts, so a double-layer slot holding two phases
    // contributes to both of their winding functions.
    for conductor in conductors {
        let sign = match conductor.direction {
            Direction::Out => 1.0_f32,
            Direction::In => -1.0_f32,
        };

        // Smallest i with `i / SAMPLES * TAU >= s / n * TAU`, computed in
        // integers to avoid the float rounding of the previous comparison.
        let step_at = (conductor.slot * WAVEFORM_SAMPLES).div_ceil(n);
        if step_at < WAVEFORM_SAMPLES {
            wf[conductor.phase][step_at] += sign;
        }
    }

    // Integrate the steps into the piecewise-constant winding function.
    for phase_wf in wf.iter_mut() {
        let mut running = 0.0_f32;
        for sample in phase_wf.iter_mut() {
            running += *sample;
            *sample = running;
        }
    }

    // Remove DC offset from each winding function (ensure mean = 0)
    for phase_wf in wf.iter_mut() {
        let mean = phase_wf.iter().sum::<f32>() / WAVEFORM_SAMPLES as f32;
        for v in phase_wf.iter_mut() {
            *v -= mean;
        }
    }

    // Instantaneous phase currents: i_k(t) = cos(elec_angle - k * alpha_m)
    let alpha_m = if m > 0 && !m.is_multiple_of(2) {
        TAU / m as f32
    } else if m > 0 {
        std::f32::consts::PI / m as f32
    } else {
        TAU
    };

    let currents: Vec<f32> = (0..m)
        .map(|k| (elec_angle - k as f32 * alpha_m).cos())
        .collect();

    // Per-phase MMF and total MMF
    let phase_mmf: Vec<Vec<f32>> = (0..m)
        .map(|k| wf[k].iter().map(|&w| w * currents[k]).collect())
        .collect();

    let total_mmf: Vec<f32> = (0..WAVEFORM_SAMPLES)
        .map(|i| (0..m).map(|k| phase_mmf[k][i]).sum::<f32>())
        .collect();

    // ── Find y-axis scale ────────────────────────────────────────────────────
    let y_max = total_mmf
        .iter()
        .cloned()
        .chain(phase_mmf.iter().flatten().cloned())
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let y_min = total_mmf
        .iter()
        .cloned()
        .chain(phase_mmf.iter().flatten().cloned())
        .fold(0.0_f32, f32::min)
        .min(-1.0);
    let y_range = (y_max - y_min).max(0.001);

    // ── Layout ───────────────────────────────────────────────────────────────
    let pad_l = 38.0;
    let pad_r = 12.0;
    let pad_t = 12.0;
    let pad_b = 24.0;

    let plot_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad_l, rect.top() + pad_t),
        egui::pos2(rect.right() - pad_r, rect.bottom() - pad_b),
    );

    // Helper: convert (sample_index ∈ [0, WAVEFORM_SAMPLES], value ∈ [y_min, y_max]) → screen pos
    let to_screen = |i: usize, v: f32| -> egui::Pos2 {
        let tx = i as f32 / WAVEFORM_SAMPLES as f32;
        let ty = (v - y_min) / y_range;
        egui::pos2(
            plot_rect.left() + tx * plot_rect.width(),
            plot_rect.bottom() - ty * plot_rect.height(),
        )
    };

    painter.rect_stroke(
        plot_rect,
        2.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)),
        egui::StrokeKind::Middle,
    );

    // Zero line
    {
        let y0 = to_screen(0, 0.0).y;
        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), y0),
                egui::pos2(plot_rect.right(), y0),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 90)),
        );
    }

    // X-axis grid lines at 0, π/2, π, 3π/2, 2π
    for tick_frac in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
        let x = plot_rect.left() + tick_frac * plot_rect.width();
        painter.line_segment(
            [
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 55)),
        );
        let label = match (tick_frac * 8.0) as u32 {
            0 => "0",
            2 => "π/2",
            4 => "π",
            6 => "3π/2",
            8 => "2π",
            _ => "",
        };
        painter.text(
            egui::pos2(x, plot_rect.bottom() + 10.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(9.0),
            egui::Color32::from_rgb(110, 110, 130),
        );
    }

    // Y-axis labels
    for &v in &[y_min, 0.0f32, y_max] {
        let y = to_screen(0, v).y;
        painter.text(
            egui::pos2(rect.left() + pad_l - 4.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{:.0}", v),
            egui::FontId::proportional(8.0),
            egui::Color32::from_rgb(110, 110, 130),
        );
    }

    for (k, mmf) in phase_mmf.iter().enumerate().take(m) {
        let color = phase::colors::phase_color_egui(k, m);
        draw_polyline(
            &painter,
            |i| to_screen(i, mmf.get(i).copied().unwrap_or(0.0)),
            WAVEFORM_SAMPLES,
            color,
            1.0,
        );
    }

    // Draw total MMF (bold green)
    let green = egui::Color32::from_rgb(60, 210, 80);
    draw_polyline(
        &painter,
        |i| to_screen(i, total_mmf.get(i).copied().unwrap_or(0.0)),
        WAVEFORM_SAMPLES,
        green,
        2.5,
    );

    // X-axis label
    painter.text(
        egui::pos2(plot_rect.center().x, rect.bottom() - 4.0),
        egui::Align2::CENTER_CENTER,
        i18n::t(lang, "mechanical_angle"),
        egui::FontId::proportional(9.0),
        egui::Color32::from_rgb(130, 130, 150),
    );

    // Legend
    {
        let mut lx = plot_rect.left();
        let ly = rect.top() + 6.0;

        for k in 0..m {
            let col = phase::colors::phase_color_egui(k, m);
            let letter = crate::phase::letter::phase_letter(k);
            let raw_label = i18n::t(lang, "phase_wf");
            let label = raw_label.replace("{}", &letter.to_string());
            painter.line_segment(
                [egui::pos2(lx, ly), egui::pos2(lx + 14.0, ly)],
                egui::Stroke::new(1.5, col),
            );
            painter.text(
                egui::pos2(lx + 16.0, ly),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::proportional(8.5),
                col,
            );
            lx += label.len() as f32 * 5.0 + 20.0;
        }

        // total mmf legend
        painter.line_segment(
            [egui::pos2(lx, ly), egui::pos2(lx + 14.0, ly)],
            egui::Stroke::new(2.5, green),
        );
        painter.text(
            egui::pos2(lx + 16.0, ly),
            egui::Align2::LEFT_CENTER,
            i18n::t(lang, "total_mmf"),
            egui::FontId::proportional(8.5),
            green,
        );
    }
}

// ─── Drawing helpers ─────────────────────────────────────────────────────────

/// Draw a polyline from a closure that maps sample index → screen position.
///
/// Emitted as a single `Shape::line` rather than one shape per segment: at 720
/// samples × (m + 1) curves the per-segment version pushed several thousand
/// shapes through the tessellator every frame.
fn draw_polyline(
    painter: &egui::Painter,
    f: impl Fn(usize) -> egui::Pos2,
    samples: usize,
    color: egui::Color32,
    width: f32,
) {
    if samples < 2 {
        return;
    }
    let points: Vec<egui::Pos2> = (0..samples).map(f).collect();
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(width, color),
    ));
}
