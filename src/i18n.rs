//! UI strings, as compile-time catalogs.
//!
//! Language is global state owned by the `i18n` crate rather than a Bevy
//! resource: `t!` reads it directly, so no system needs to carry the current
//! language down to the widget that renders a string.

use bevy::prelude::*;
pub use i18n::{Languages, current_language, set_language};

/// The language the panel starts in.
pub const DEFAULT_LANGUAGE: Languages = Languages::PtBr;

/// The languages offered in the selector, in display order.
///
/// Only these two have catalogs — `Languages::ALL` also lists `Pt` and `EnUs`,
/// whose `lang-*` features are off, and offering one would blank the whole UI.
pub const OFFERED: [Languages; 2] = [Languages::PtBr, Languages::En];

/// Two-letter tag for the selector button.
pub fn short_tag(lang: Languages) -> &'static str {
    match lang {
        Languages::PtBr | Languages::Pt => "PT",
        Languages::En | Languages::EnUs => "EN",
    }
}

/// Publishes [`DEFAULT_LANGUAGE`] — the `i18n` crate's own default is `En`.
pub struct I18nPlugin;

impl Plugin for I18nPlugin {
    fn build(&self, _app: &mut App) {
        i18n::set_language(DEFAULT_LANGUAGE);
    }
}

i18n::traductions! {
    pub enum Strings {
        MotorConfigBtn([En("Motor Config"), PtBr("Config. Motor")]),
        MotorConfigHeading([En("Motor Configuration"), PtBr("Configuração do Motor")]),
        MinimizePanelHover([En("Minimize Panel"), PtBr("Minimizar Painel")]),
        Language([En("Language"), PtBr("Idioma")]),

        ElectricalCurrents([En("Electrical Currents"), PtBr("Correntes Elétricas")]),
        Play([En("Play"), PtBr("Iniciar")]),
        Pause([En("Pause"), PtBr("Pausar")]),
        Speed([En("Speed"), PtBr("Velocidade")]),

        Grooves([En("Grooves (slots)"), PtBr("Ranhuras (cavidades)")]),
        Phases([En("Phases"), PtBr("Fases")]),
        Phase([En("Phase"), PtBr("Fase")]),
        Poles([En("Poles"), PtBr("Polos")]),
        Layers([En("Layers"), PtBr("Camadas")]),

        ShortPitched([En("Short-pitched coils"), PtBr("Encurtamento de passo")]),
        ShortPitchedNeedsLayers([
            En("Chording needs two electrical layers — that is what puts different \
                phases in one slot. With 1 layer the span does not change the MMF \
                (pitch factor = 1)."),
            PtBr("O encurtamento exige duas camadas elétricas: é o que coloca fases \
                  diferentes na mesma ranhura. Com 1 camada o passo não altera a FMM \
                  (fator de passo = 1).")
        ]),

        ShowHeaders([En("Show coil headers"), PtBr("Mostrar cabeças de bobina")]),
        ToggleHeadersHover([
            En("Toggle visibility of endwinding arcs"),
            PtBr("Alternar visibilidade dos arcos de enrolamento")
        ]),
        ShowVectors([En("Show MMF vectors"), PtBr("Mostrar vetores FMM")]),
        ShowRotor([En("Show rotor"), PtBr("Mostrar rotor")]),

        ValidConfig([En("Valid configuration"), PtBr("Configuração válida")]),
        InvalidConfig([En("Invalid configuration"), PtBr("Configuração inválida")]),
        InvalidConfigHint([
            En("Grooves must be divisible by 2 × poles × phases."),
            PtBr("As ranhuras devem ser divisíveis por 2 × polos × fases.")
        ]),

        RotateHint([En("Left-click drag to rotate"), PtBr("Botão esquerdo para rotacionar")]),
        ZoomHint([En("Scroll to zoom"), PtBr("Scroll para zoom")]),

        DistributionIndex([En("Distribution index"), PtBr("Índice de distribuição")]),
        SlotAngle([En("Angle between slots"), PtBr("Ângulo entre ranhuras")]),
        PhaseAngle([En("Angle between phases"), PtBr("Ângulo entre fases")]),
        SlotsPerPole([En("Slots per pole"), PtBr("Ranhuras por polo")]),
        TotalPoles([En("Total poles"), PtBr("Total de polos")]),

        ShowMmfField([En("Show MMF field"), PtBr("Mostrar campo FMM")]),
        ToggleMmfFieldHover([
            En("Toggle visibility of MMF field"),
            PtBr("Alternar visibilidade do campo FMM")
        ]),
        MmfGradientIntensity([En("Gradient intensity (γ)"), PtBr("Intensidade do gradiente (γ)")]),
        MmfGradientIntensityHover([
            En("Controls gradient sharpness: higher values concentrate the field \
                more towards the magnetic axis"),
            PtBr("Controla a concentração do gradiente de campo: valores maiores \
                  deixam o campo mais concentrado no eixo magnético")
        ]),
        MmfResult([En("Result"), PtBr("Resultado")]),

        ShowWindingScheme([En("Show Winding Scheme"), PtBr("Mostrar Esquema de Enrolamento")]),
        ShowWindingSchemeHover([
            En("Opens a window with the winding diagram and MMF waveforms"),
            PtBr("Abre uma janela com o diagrama de enrolamento e as formas de onda da FMM")
        ]),
        WindingSchemeTitle([En("Winding Scheme"), PtBr("Esquema de Enrolamento")]),
        WindingFunctionMmf([En("Winding Function & MMF"), PtBr("Função de Enrolamento & FMM")]),
        MechanicalAngle([En("mechanical angle αs (rad)"), PtBr("ângulo mecânico αs (rad)")]),
        PhaseWf([En("{phase}-phase WF"), PtBr("fase-{phase} WF")]),
        TotalMmf([En("total mmf"), PtBr("fmm total")]),
    }
}

// The winding-factor readouts and the spectrum panel exist only with the
// `harmonics` feature, so their catalogs are declared in their own enum — a
// `traductions!` body takes no `cfg` attributes on individual variants, and
// gating the whole invocation is what keeps the strings out of a lean build.
#[cfg(feature = "harmonics")]
i18n::traductions! {
    pub enum HarmonicStrings {
        DistributionFactor([En("Distribution factor"), PtBr("Fator de distribuição")]),
        DistributionFactorHover([
            En("Spreading a phase over q slots adds the coil EMFs as a fan of \
                phasors rather than head to tail. k_d = sin(qγ/2) / (q·sin(γ/2))."),
            PtBr("Espalhar a fase por q ranhuras soma as FEMs como um leque de \
                  fasores, não em linha reta. k_d = sen(qγ/2) / (q·sen(γ/2)).")
        ]),
        PitchFactor([En("Pitch factor"), PtBr("Fator de passo")]),
        PitchFactorHover([
            En("Exactly 1 at full pitch. Only chording lowers it — and that is what \
                cuts the 5th and 7th harmonics. k_p = sin(ν·(y/τ)·90°)."),
            PtBr("Vale 1 com passo pleno. Só o encurtamento o reduz — e é essa \
                  redução que corta o 5º e o 7º harmônicos. k_p = sen(ν·(y/τ)·90°).")
        ]),
        WindingFactor([En("Winding factor"), PtBr("Fator de enrolamento")]),
        WindingFactorHover([
            En("The fraction of the ideal (concentrated, full-pitch) MMF this \
                winding actually produces at the fundamental."),
            PtBr("Fração da FMM ideal (concentrada e de passo pleno) que este \
                  enrolamento realmente produz na fundamental.")
        ]),
        HarmonicSpectrum([
            En("Harmonic spectrum (% of fundamental)"),
            PtBr("Espectro harmônico (% da fundamental)")
        ]),
    }
}
