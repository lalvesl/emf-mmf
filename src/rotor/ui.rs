use crate::config::{RotorConfig, ViewConfig};
use crate::i18n::Strings;
use crate::rotor::torque::RotorEquilibrium;
use crate::ui::{float_slider, int_slider, slider_caption, toggle_row};
use bevy_egui::egui;
use egui_sc::egui_components::{Alert, AlertVariant, Boxed, Spacing, Tooltip, muted_text};
use i18n::t;

pub fn rotor_ui(
    ui: &mut egui::Ui,
    equilibrium: &RotorEquilibrium,
    view: &mut ViewConfig,
    rotor: &mut RotorConfig,
) {
    toggle_row(ui, &mut view.show_rotor, &t!(Strings::ShowRotor), None);

    // Nothing below says anything about a rotor that is not on screen.
    if !view.show_rotor {
        return;
    }

    Spacing::Xs.show(ui);

    // The flag decides whether the rotor is allowed a count of its own; the
    // slider's value is kept while it is off, so toggling back on restores the
    // machine that was being demonstrated rather than resetting it.
    toggle_row(
        ui,
        &mut rotor.independent_poles,
        &t!(Strings::RotorIndependentPoles),
        Some(&t!(Strings::RotorIndependentPolesHover)),
    );

    if rotor.independent_poles {
        Spacing::Xs.show(ui);
        rotor_poles(ui, rotor);
    }

    Spacing::Xs.show(ui);
    load_angle(ui, rotor);

    Spacing::Sm.show(ui);
    alignment_readout(ui, equilibrium, rotor.load_angle);
}

/// Rotor pole count. Even like the stator's, so the slider steps in pairs.
fn rotor_poles(ui: &mut egui::Ui, rotor: &mut RotorConfig) {
    slider_caption(
        ui,
        &format!(
            "{} (P_r): {}",
            t!(Strings::RotorPoles),
            rotor.pole_pairs * 2
        ),
    );

    let mut poles = rotor.pole_pairs * 2;
    int_slider(
        ui,
        "rotor_poles",
        &mut poles,
        RotorConfig::MIN.pole_pairs * 2,
        RotorConfig::MAX.pole_pairs * 2,
        2,
    );
    rotor.pole_pairs = poles / 2;
}

/// The angle the rotor is held at, away from alignment with the stator flux.
///
/// Degrees on the way in and back out: the caption is read off the panel, and
/// a load angle in radians is unreadable there. The round trip is stable —
/// the slider snaps to whole degrees, which survive both conversions — so it
/// cannot mark the resource changed on a frame where nothing moved.
fn load_angle(ui: &mut egui::Ui, rotor: &mut RotorConfig) {
    let mut degrees = rotor.load_angle.to_degrees();

    Tooltip::new(&t!(Strings::LoadAngleHover)).wrap(ui, |ui| {
        ui.scope(|ui| {
            slider_caption(ui, &format!("{}: {degrees:+.0}°", t!(Strings::LoadAngle)));
        })
        .response
    });

    float_slider(
        ui,
        "load_angle",
        &mut degrees,
        RotorConfig::MIN.load_angle.to_degrees(),
        RotorConfig::MAX.load_angle.to_degrees(),
        1.0,
    );

    rotor.load_angle = degrees.to_radians();
}

/// What the field actually does to this rotor: where it holds it, how firmly,
/// and where on the torque curve the commanded angle has put it.
fn alignment_readout(ui: &mut egui::Ui, equilibrium: &RotorEquilibrium, load_angle: f32) {
    let positions = equilibrium.positions.len();
    let grip = equilibrium.grip * 100.0;

    Boxed::new()
        .padding(Spacing::Sm)
        .accent(true)
        .show(ui, |ui| {
            Tooltip::new(&t!(Strings::StablePositionsHover)).wrap(ui, |ui| {
                ui.scope(|ui| {
                    let span = equilibrium
                        .well_span()
                        .map(|span| format!(" ({:.0}°)", span.to_degrees()))
                        .unwrap_or_default();
                    muted_text(
                        ui,
                        &format!("{}: {positions}{span}", t!(Strings::StablePositions)),
                    );
                })
                .response
            });

            Tooltip::new(&t!(Strings::AvailableTorqueHover)).wrap(ui, |ui| {
                ui.scope(|ui| {
                    muted_text(ui, &format!("{}: {grip:.0}%", t!(Strings::AvailableTorque)));
                })
                .response
            });

            // With no wells there is no curve to be on, and quoting a torque
            // would be inventing one.
            if positions > 0 {
                muted_text(
                    ui,
                    &format!(
                        "{}: {:+.0}% (δ {:+.0}°)",
                        t!(Strings::AlignmentTorque),
                        equilibrium.torque(load_angle) * 100.0,
                        equilibrium.settle(load_angle).to_degrees()
                    ),
                );
            }

            // Past a quarter well the restoring torque falls off, so the rotor
            // is on its way to the next position whether or not the slider
            // stops moving. Worth saying out loud — it is the whole phenomenon.
            if equilibrium.past_pullout(load_angle) {
                Spacing::Xs.show(ui);
                Alert::new(&t!(Strings::PastPullout))
                    .variant(AlertVariant::Warning)
                    .show(ui);
            }

            // Two different things, and the panel must not say the first when
            // it means the second: a rotor that settles but pulls weakly has
            // *lost* torque, while one the field cancels on has none at all.
            if positions == 0 {
                Spacing::Xs.show(ui);
                Alert::new(&t!(Strings::NoPreferredPosition))
                    .variant(AlertVariant::Warning)
                    .show(ui);
            } else if equilibrium.grip < 0.99 {
                Spacing::Xs.show(ui);
                Alert::new(&format!("{} {grip:.1}%", t!(Strings::WeakTorque)))
                    .description(&t!(Strings::WeakTorqueHint))
                    .variant(AlertVariant::Warning)
                    .show(ui);
            }
        });
}
