use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use egui_sc::egui_components::{Separator, ShadcnTheme, Spacing, heading4};
use i18n::t;
#[cfg(feature = "harmonics")]
use std::f32::consts::{PI, TAU};

use crate::config::{MotorConfig, ViewConfig};
use crate::electrical::ElectricalState;
use crate::i18n::Strings;
use crate::phase;
use crate::ui::PanelLayout;
use crate::winding::{
    Conductor, Direction, SlotPacking, axis, compute_conductors, compute_winding,
};

pub struct WindingSchemePlugin;

impl Plugin for WindingSchemePlugin {
    fn build(&self, app: &mut App) {
        // After the theme stage: every component and painter below reads the
        // theme out of context memory, which is empty until it is published.
        app.add_systems(
            EguiPrimaryContextPass,
            winding_scheme_window
                .after(PanelLayout::Theme)
                .run_if(crate::theme::fonts_ready),
        );
    }
}

/// How many angular samples to use when drawing the MMF step waveforms.
const WAVEFORM_SAMPLES: usize = 720;

fn winding_scheme_window(
    mut contexts: EguiContexts,
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    state: Res<ElectricalState>,
) {
    if !view.show_winding_scheme {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new(t!(Strings::WindingSchemeTitle).into_owned())
        .id(egui::Id::new("winding_scheme_window"))
        .default_width(620.0)
        .min_width(400.0)
        .resizable(true)
        .show(ctx, |ui| {
            let assignments = compute_winding(&config);
            let conductors = compute_conductors(&config, &assignments);

            // ── Top panel: conductor layout ─────────────────────────────────
            // Height grows with the stack so a 6-conductor slot still fits.
            let rows = SlotPacking::new(config.layers).rows;
            let conductor_panel_height = 46.0 + rows as f32 * 22.0;
            let (rect_top, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), conductor_panel_height),
                egui::Sense::hover(),
            );
            draw_conductor_panel(ui, rect_top, &config, &conductors);

            Spacing::Xs.show(ui);
            Separator::horizontal().show(ui);
            Spacing::Xs.show(ui);

            // ── Bottom panel: MMF waveforms ─────────────────────────────────
            heading4(ui, &t!(Strings::WindingFunctionMmf));
            let waveform_panel_height = 180.0_f32;
            let (rect_bot, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), waveform_panel_height),
                egui::Sense::hover(),
            );
            draw_mmf_panel(ui, rect_bot, &config, &conductors, state.angle);

            spectrum_section(ui, &config, &conductors);
        });
}

/// The harmonic spectrum section of the window.
#[cfg(feature = "harmonics")]
fn spectrum_section(ui: &mut egui::Ui, config: &MotorConfig, conductors: &[Conductor]) {
    use crate::i18n::HarmonicStrings;

    Spacing::Xs.show(ui);
    Separator::horizontal().show(ui);
    Spacing::Xs.show(ui);
    heading4(ui, &t!(HarmonicStrings::HarmonicSpectrum));
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 150.0),
        egui::Sense::hover(),
    );
    draw_spectrum_panel(ui, rect, config, conductors);
}

#[cfg(not(feature = "harmonics"))]
fn spectrum_section(_: &mut egui::Ui, _: &MotorConfig, _: &[Conductor]) {}

/// Highest electrical harmonic the spectrum shows.
#[cfg(feature = "harmonics")]
const MAX_HARMONIC: usize = 15;

/// Amplitude of each odd electrical harmonic of the winding function.
///
/// The winding function of phase 0 is sampled around the bore and transformed
/// directly, so the result reflects the conductors as they are actually laid
/// out — including the two electrical layers landing on different phases when
/// the coils are chorded. No closed form is assumed, which also means this
/// stays right for windings the textbook `k_d`/`k_p` formulas do not cover.
///
/// A `p`-pole-pair machine turns electrical harmonic `ν` into spatial order
/// `p·ν`, so that is the order each one is read at.
#[cfg(feature = "harmonics")]
fn winding_function_spectrum(config: &MotorConfig, conductors: &[Conductor]) -> Vec<(usize, f32)> {
    let n = config.groove_count;
    let p = config.pole_pairs.max(1);
    if n == 0 {
        return Vec::new();
    }

    // Piecewise-constant winding function of phase 0, as a running sum of the
    // unit steps each conductor contributes at its slot.
    let mut steps = vec![0.0_f32; n];
    for conductor in conductors.iter().filter(|c| c.phase == 0) {
        steps[conductor.slot] += match conductor.direction {
            Direction::Out => 1.0,
            Direction::In => -1.0,
        };
    }

    let mut function = Vec::with_capacity(n);
    let mut running = 0.0_f32;
    for step in &steps {
        running += step;
        function.push(running);
    }

    // Each step is integrated in closed form rather than the function being
    // sampled and transformed discretely. Sampling once per slot would cap the
    // readable orders at `n/2` — with 24 slots and two pole pairs that stops at
    // the fifth, and the seventh would fold back onto it. A staircase has exact
    // coefficients at every order, so there is no reason to accept that.
    //
    // Over one step `∫e^{-iνθ}dθ = (e^{-iνθ₁} − e^{-iνθ₀}) / (−iν)`, and the
    // constant term telescopes away on its own — no DC removal needed.
    let step_angle = TAU / n as f32;
    let amplitude_at = |order: usize| -> f32 {
        if order == 0 {
            return 0.0;
        }
        let (mut re, mut im) = (0.0_f32, 0.0_f32);
        for (slot, &value) in function.iter().enumerate() {
            let a0 = order as f32 * slot as f32 * step_angle;
            let a1 = a0 + order as f32 * step_angle;
            re += value * (a1.cos() - a0.cos());
            im += value * (a0.sin() - a1.sin());
        }
        (re * re + im * im).sqrt() / (PI * order as f32)
    };

    // Even harmonics vanish in any symmetric winding, so only odd ones are
    // worth the width on screen.
    (1..=MAX_HARMONIC)
        .step_by(2)
        .map(|harmonic| (harmonic, amplitude_at(harmonic * p)))
        .collect()
}

/// Radius of a conductor symbol and the pitch between the columns of one slot.
///
/// Sized from the share of the width a *single* conductor gets, which is the
/// slot pitch divided by the conductors packed side by side in it. Sizing from
/// the slot pitch alone gave every symbol nearly the whole slot to itself, so a
/// two-column slot drew its pair on top of each other and the row spilled over
/// its neighbours.
fn conductor_symbol(slot_step: f32, row_step: f32, cols: usize) -> (f32, f32) {
    /// How much of a slot's pitch the symbols may take, leaving the rest as the
    /// gap that keeps neighbouring slots readable as separate.
    const SLOT_FILL: f32 = 0.9;
    /// Column pitch as a multiple of the symbol diameter — just over 1, so the
    /// two columns of a slot touch but never overlap.
    const COL_PITCH: f32 = 1.08;

    let cols = cols.max(1) as f32;
    let span = (cols - 1.0) * COL_PITCH + 1.0;
    let radius = (slot_step * SLOT_FILL / (2.0 * span))
        .min(row_step * 0.42)
        .max(0.0);

    (radius, radius * 2.0 * COL_PITCH)
}

fn draw_conductor_panel(
    ui: &egui::Ui,
    rect: egui::Rect,
    config: &MotorConfig,
    conductors: &[crate::winding::Conductor],
) {
    let theme = ShadcnTheme::get(ui.ctx());
    let painter = ui.painter_at(rect);
    let n = config.groove_count;

    let cy = rect.bottom() - 8.0;
    // Spread the n slots over the full rect width with some padding
    let padding = 24.0;
    let slot_step = (rect.width() - padding * 2.0) / n as f32;

    // The same packing the 3D view uses, not a second copy of the arithmetic.
    let packing = SlotPacking::new(config.layers);

    // The cap on how tall a row may be; the symbol takes less when the slots
    // are narrow enough to shrink it.
    let max_row_step = 20.0_f32;
    let (sym_r, col_step) = conductor_symbol(slot_step, max_row_step, packing.cols);
    // Rows follow the symbol rather than sitting at a fixed pitch, or a stack
    // of small symbols reads as disconnected dots adrift in a tall panel.
    let row_step = (sym_r * 2.0 * 1.15).min(max_row_step);

    // Tick marks, one per slot
    for s in 0..n {
        let x = rect.left() + padding + (s as f32 + 0.5) * slot_step;
        painter.line_segment(
            [egui::pos2(x, cy), egui::pos2(x, cy - 5.0)],
            egui::Stroke::new(1.0, theme.border),
        );
    }

    // Draw conductor symbols, stacked so both electrical layers are visible
    for conductor in conductors {
        let slot_x = rect.left() + padding + (conductor.slot as f32 + 0.5) * slot_step;

        let (row, _) = packing.row_col(conductor.index);

        // Row 0 is the slot bottom, drawn at the top of the stack.
        let x = slot_x + packing.column_offset(conductor.index) * col_step;
        let y = cy - 12.0 - (packing.rows - 1 - row) as f32 * row_step;

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
) {
    let theme = ShadcnTheme::get(ui.ctx());
    let painter = ui.painter_at(rect);
    let n = config.groove_count;
    let m = config.phases;

    // Background
    painter.rect_filled(rect, plot_corner(&theme), theme.muted);

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
    let alpha_m = axis::phase_displacement(m);
    let currents: Vec<f32> = (0..m)
        .map(|k| axis::phase_current(elec_angle, k, alpha_m))
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
        egui::Stroke::new(1.0, theme.border),
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
            egui::Stroke::new(1.0, axis_color(&theme)),
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
            egui::Stroke::new(1.0, grid_color(&theme)),
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
            theme.muted_foreground,
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
            theme.muted_foreground,
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

    // White, the colour the resultant already carries in the 3D overlay and in
    // the field legend. `foreground` rather than a literal, so it survives a
    // light theme — nothing else in this plot uses it, so it still stands out
    // against the phase curves.
    let total_color = theme.foreground;
    draw_polyline(
        &painter,
        |i| to_screen(i, total_mmf.get(i).copied().unwrap_or(0.0)),
        WAVEFORM_SAMPLES,
        total_color,
        2.5,
    );

    // X-axis label
    painter.text(
        egui::pos2(plot_rect.center().x, rect.bottom() - 4.0),
        egui::Align2::CENTER_CENTER,
        t!(Strings::MechanicalAngle),
        egui::FontId::proportional(9.0),
        theme.muted_foreground,
    );

    // Legend
    {
        let mut lx = plot_rect.left();
        let ly = rect.top() + 6.0;

        for k in 0..m {
            let col = phase::colors::phase_color_egui(k, m);
            let letter = crate::phase::letter::phase_letter(k);
            let label = t!(Strings::PhaseWf, phase = letter.to_string());
            painter.line_segment(
                [egui::pos2(lx, ly), egui::pos2(lx + 14.0, ly)],
                egui::Stroke::new(1.5, col),
            );
            painter.text(
                egui::pos2(lx + 16.0, ly),
                egui::Align2::LEFT_CENTER,
                label.as_ref(),
                egui::FontId::proportional(8.5),
                col,
            );
            lx += label.len() as f32 * 5.0 + 20.0;
        }

        // total mmf legend
        painter.line_segment(
            [egui::pos2(lx, ly), egui::pos2(lx + 14.0, ly)],
            egui::Stroke::new(2.5, total_color),
        );
        painter.text(
            egui::pos2(lx + 16.0, ly),
            egui::Align2::LEFT_CENTER,
            t!(Strings::TotalMmf),
            egui::FontId::proportional(8.5),
            total_color,
        );
    }
}

// ─── Drawing helpers ─────────────────────────────────────────────────────────

/// Corner radius for a plot surface, from the theme's own radius.
fn plot_corner(theme: &ShadcnTheme) -> egui::CornerRadius {
    egui::CornerRadius::same(theme.radius as u8)
}

/// The zero line and other axes a reader should notice but not read past.
fn axis_color(theme: &ShadcnTheme) -> egui::Color32 {
    ShadcnTheme::with_alpha(theme.muted_foreground, 90)
}

/// Reference gridlines — present, but never competing with the curves.
fn grid_color(theme: &ShadcnTheme) -> egui::Color32 {
    ShadcnTheme::with_alpha(theme.muted_foreground, 40)
}

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
    painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
}

// ─── Harmonic spectrum panel ──────────────────────────────────────────────────

#[cfg(feature = "harmonics")]
fn draw_spectrum_panel(
    ui: &egui::Ui,
    rect: egui::Rect,
    config: &MotorConfig,
    conductors: &[Conductor],
) {
    let theme = ShadcnTheme::get(ui.ctx());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, plot_corner(&theme), theme.muted);

    let spectrum = winding_function_spectrum(config, conductors);
    // An invalid configuration produces no conductors at all, so there is no
    // fundamental to measure the rest against — leave the panel blank rather
    // than dividing by it.
    let Some(&(_, fundamental)) = spectrum.first() else {
        return;
    };
    if !fundamental.is_finite() || fundamental <= f32::EPSILON {
        return;
    }

    let pad_l = 34.0;
    let pad_r = 12.0;
    let pad_t = 10.0;
    let pad_b = 22.0;
    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad_l, rect.top() + pad_t),
        egui::pos2(rect.right() - pad_r, rect.bottom() - pad_b),
    );

    painter.rect_stroke(
        plot,
        2.0,
        egui::Stroke::new(1.0, theme.border),
        egui::StrokeKind::Middle,
    );

    // Everything is read against the fundamental, which is the comparison that
    // matters: chording is judged by how far it pushes the 5th and 7th down
    // relative to the useful component, not by their absolute size.
    let bars = spectrum.len().max(1);
    let slot_w = plot.width() / bars as f32;
    let bar_w = (slot_w * 0.55).min(26.0);

    // Reference gridlines at 25%, 50%, 75%, 100%.
    for frac in [0.25_f32, 0.5, 0.75, 1.0] {
        let y = plot.bottom() - frac * plot.height();
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            egui::Stroke::new(1.0, grid_color(&theme)),
        );
        painter.text(
            egui::pos2(rect.left() + pad_l - 5.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{:.0}%", frac * 100.0),
            egui::FontId::proportional(9.0),
            theme.muted_foreground,
        );
    }

    for (index, &(harmonic, amplitude)) in spectrum.iter().enumerate() {
        let ratio = (amplitude / fundamental).clamp(0.0, 1.0);
        let cx = plot.left() + (index as f32 + 0.5) * slot_w;
        let top = plot.bottom() - ratio * plot.height();

        // The fundamental is the reference; the 5th and 7th are the pair that
        // chording exists to suppress, so they are called out.
        let color = match harmonic {
            1 => theme.success,
            5 | 7 => theme.warning,
            _ => theme.muted_foreground,
        };

        let bar = egui::Rect::from_min_max(
            egui::pos2(cx - bar_w * 0.5, top),
            egui::pos2(cx + bar_w * 0.5, plot.bottom()),
        );
        painter.rect_filled(bar, 1.0, color);

        painter.text(
            egui::pos2(cx, plot.bottom() + 10.0),
            egui::Align2::CENTER_CENTER,
            harmonic.to_string(),
            egui::FontId::proportional(9.0),
            theme.muted_foreground,
        );

        // The harmonics sit far below the fundamental once the `1/ν` of the
        // integration is in, so the number carries the reading where the bar
        // is too short to.
        if ratio > 0.004 {
            painter.text(
                egui::pos2(cx, top - 6.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{:.0}", ratio * 100.0),
                egui::FontId::proportional(9.0),
                color,
            );
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "harmonics"))]
mod tests {
    use super::*;

    fn config(groove_count: usize, phases: usize, pole_pairs: usize, layers: usize) -> MotorConfig {
        MotorConfig {
            groove_count,
            phases,
            pole_pairs,
            layers,
            ..default()
        }
    }

    fn spectrum(config: &MotorConfig) -> Vec<(usize, f32)> {
        let assignments = compute_winding(config);
        let conductors = compute_conductors(config, &assignments);
        winding_function_spectrum(config, &conductors)
    }

    /// Amplitude of `harmonic` relative to the fundamental.
    fn relative(spectrum: &[(usize, f32)], harmonic: usize) -> f32 {
        let fundamental = spectrum[0].1;
        let (_, amplitude) = spectrum
            .iter()
            .find(|(h, _)| *h == harmonic)
            .unwrap_or_else(|| panic!("harmonic {harmonic} is missing from the spectrum"));
        amplitude / fundamental
    }

    /// The transform of the real conductor layout must agree with the closed
    /// form it is meant to make visible. If these two ever disagree, one of
    /// them is wrong — and the numeric one is the honest witness, since it
    /// makes no assumption about the winding being regular.
    #[test]
    fn the_spectrum_agrees_with_the_closed_form_winding_factors() {
        for short_pitched in [false, true] {
            let mut cfg = config(24, 3, 2, 2);
            cfg.short_pitched = short_pitched;
            let spec = spectrum(&cfg);

            for harmonic in [5_usize, 7, 11] {
                let numeric = relative(&spec, harmonic);
                let closed = (axis::winding_factor(&cfg, harmonic) / axis::winding_factor(&cfg, 1))
                    .abs()
                    / harmonic as f32;

                assert!(
                    (numeric - closed).abs() < 0.02,
                    "short_pitched={short_pitched} ν={harmonic}: \
                     DFT says {numeric:.4}, closed form says {closed:.4}"
                );
            }
        }
    }

    /// The payoff chording is for: the fifth and seventh collapse, and the
    /// fundamental barely moves.
    #[test]
    fn chording_suppresses_the_fifth_and_seventh() {
        let mut full = config(24, 3, 2, 2);
        full.short_pitched = false;
        let mut chorded = full.clone();
        chorded.short_pitched = true;

        let spec_full = spectrum(&full);
        let spec_chorded = spectrum(&chorded);

        for harmonic in [5_usize, 7] {
            let before = relative(&spec_full, harmonic);
            let after = relative(&spec_chorded, harmonic);
            assert!(
                after < before * 0.5,
                "ν={harmonic}: chording only took {before:.3} to {after:.3}"
            );
        }

        // Meanwhile the fundamental keeps almost all of its amplitude.
        let kept = spec_chorded[0].1 / spec_full[0].1;
        assert!(
            (0.9..1.0).contains(&kept),
            "the fundamental lost too much: kept {kept:.3}"
        );
    }

    /// Integrating the steps in closed form has no Nyquist ceiling, so every
    /// requested harmonic must come back — including the ones a once-per-slot
    /// transform would have folded onto a lower order. With `S=24, p=2` the
    /// seventh needs spatial order 14 against only 24 slots, which is exactly
    /// the case that used to alias.
    #[test]
    fn every_odd_harmonic_is_reported_however_few_slots() {
        for grooves in [12_usize, 24, 48] {
            for pole_pairs in 1..=6_usize {
                let cfg = config(grooves, 3, pole_pairs, 2);
                let spec = spectrum(&cfg);
                let harmonics: Vec<_> = spec.iter().map(|(h, _)| *h).collect();
                assert_eq!(
                    harmonics,
                    (1..=MAX_HARMONIC).step_by(2).collect::<Vec<_>>(),
                    "S={grooves} p={pole_pairs}"
                );
                for (harmonic, amplitude) in spec {
                    assert!(
                        amplitude.is_finite() && amplitude >= 0.0,
                        "S={grooves} p={pole_pairs} ν={harmonic}: {amplitude}"
                    );
                }
            }
        }
    }

    /// Triplen harmonics are what the phase belts are arranged to cancel, so a
    /// three-phase winding must show the third well below the fundamental.
    #[test]
    fn the_third_stays_below_the_fundamental() {
        // 72 slots keep `S` divisible by `2·p·m` for every pole count here, so
        // the winding is actually valid rather than silently empty.
        for pole_pairs in [1_usize, 2, 3, 4, 6] {
            let cfg = config(72, 3, pole_pairs, 2);
            let third = relative(&spectrum(&cfg), 3);
            assert!(
                third < 0.4,
                "p={pole_pairs}: third is {third:.3} of the fundamental"
            );
        }
    }

    /// An invalid configuration has no conductors, so the spectrum must come
    /// back flat instead of dividing by an absent fundamental.
    #[test]
    fn an_invalid_configuration_yields_a_flat_spectrum() {
        // 24 slots cannot host three pole pairs of a three-phase winding:
        // `2·p·m = 18` does not divide 24.
        let cfg = config(24, 3, 3, 2);
        assert!(
            !cfg.groove_count
                .is_multiple_of(2 * cfg.pole_pairs * cfg.phases)
        );

        for (harmonic, amplitude) in spectrum(&cfg) {
            assert_eq!(amplitude, 0.0, "ν={harmonic} should be silent");
        }
    }

    /// A symmetric winding has no even harmonics and no DC, so the fundamental
    /// must be the largest thing in the spectrum.
    #[test]
    fn the_fundamental_dominates() {
        for pole_pairs in [1_usize, 2, 3, 4, 6] {
            let cfg = config(72, 3, pole_pairs, 2);
            let spec = spectrum(&cfg);
            let fundamental = spec[0].1;
            assert!(fundamental > 0.0, "p={pole_pairs}: no fundamental");
            for &(harmonic, amplitude) in &spec[1..] {
                assert!(
                    amplitude <= fundamental,
                    "p={pole_pairs}: ν={harmonic} ({amplitude}) beats the fundamental"
                );
            }
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    /// Regression: the column pitch used to be one *radius* while a symbol is
    /// two radii across, so a slot holding two conductors drew them on top of
    /// each other and the row bled over its neighbours. Every conductor of a
    /// slot has to fit inside that slot's share of the width.
    #[test]
    fn conductor_symbols_fit_inside_their_slot() {
        const ROW_STEP: f32 = 20.0;
        const PADDING: f32 = 24.0;

        for width in [320.0_f32, 620.0, 1200.0] {
            for slots in [6_usize, 12, 24, 48, 144] {
                let slot_step = (width - PADDING * 2.0) / slots as f32;

                for layers in 1..=6_usize {
                    let packing = SlotPacking::new(layers);
                    let (radius, col_step) = conductor_symbol(slot_step, ROW_STEP, packing.cols);

                    if packing.cols > 1 {
                        assert!(
                            col_step >= radius * 2.0,
                            "w={width} S={slots} layers={layers}: columns sit {col_step} \
                             apart but are {} across",
                            radius * 2.0
                        );
                    }

                    // The conductor furthest from the slot axis decides how
                    // wide the whole stack draws.
                    let furthest = (0..layers)
                        .map(|i| packing.column_offset(i).abs())
                        .fold(0.0_f32, f32::max);
                    let span = 2.0 * (furthest * col_step + radius);
                    assert!(
                        span <= slot_step + 1e-3,
                        "w={width} S={slots} layers={layers}: the stack spans {span} \
                         across a {slot_step} slot"
                    );

                    assert!(
                        radius * 2.0 <= ROW_STEP,
                        "w={width} S={slots} layers={layers}: rows overlap"
                    );
                }
            }
        }
    }
}
