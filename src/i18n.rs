use bevy::prelude::*;

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum Language {
    #[default]
    PtBr,
    En,
}

pub fn t(lang: &Language, key: &str) -> &'static str {
    match (lang, key) {
        (Language::PtBr, "motor_config_btn") => "Config. Motor",
        (Language::En, "motor_config_btn") => "Motor Config",

        (Language::PtBr, "motor_config_heading") => "Configuração do Motor",
        (Language::En, "motor_config_heading") => "Motor Configuration",

        (Language::PtBr, "minimize_panel_hover") => "Minimizar Painel",
        (Language::En, "minimize_panel_hover") => "Minimize Panel",

        (Language::PtBr, "electrical_currents") => "Correntes Elétricas",
        (Language::En, "electrical_currents") => "Electrical Currents",

        (Language::PtBr, "play") => "▶ Iniciar",
        (Language::En, "play") => "▶ Play",

        (Language::PtBr, "pause") => "⏸ Pausar",
        (Language::En, "pause") => "⏸ Pause",

        (Language::PtBr, "speed") => "Velocidade",
        (Language::En, "speed") => "Speed",

        (Language::PtBr, "grooves") => "Ranhuras (cavidades)",
        (Language::En, "grooves") => "Grooves (slots)",

        (Language::PtBr, "phases") => "Fases",
        (Language::En, "phases") => "Phases",

        (Language::PtBr, "phase") => "Fase",
        (Language::En, "phase") => "Phase",

        (Language::PtBr, "poles") => "Polos",
        (Language::En, "poles") => "Poles",

        (Language::PtBr, "layers") => "Camadas",
        (Language::En, "layers") => "Layers",

        #[cfg(feature = "harmonics")]
        (Language::PtBr, "distribution_factor") => "Fator de distribuição",
        #[cfg(feature = "harmonics")]
        (Language::En, "distribution_factor") => "Distribution factor",
        #[cfg(feature = "harmonics")]
        (Language::PtBr, "distribution_factor_hover") => {
            "Espalhar a fase por q ranhuras soma as FEMs como um leque de \
             fasores, não em linha reta. k_d = sen(qγ/2) / (q·sen(γ/2))."
        }
        #[cfg(feature = "harmonics")]
        (Language::En, "distribution_factor_hover") => {
            "Spreading a phase over q slots adds the coil EMFs as a fan of \
             phasors rather than head to tail. k_d = sin(qγ/2) / (q·sin(γ/2))."
        }

        #[cfg(feature = "harmonics")]
        (Language::PtBr, "pitch_factor") => "Fator de passo",
        #[cfg(feature = "harmonics")]
        (Language::En, "pitch_factor") => "Pitch factor",
        #[cfg(feature = "harmonics")]
        (Language::PtBr, "pitch_factor_hover") => {
            "Vale 1 com passo pleno. Só o encurtamento o reduz — e é essa \
             redução que corta o 5º e o 7º harmônicos. k_p = sen(ν·(y/τ)·90°)."
        }
        #[cfg(feature = "harmonics")]
        (Language::En, "pitch_factor_hover") => {
            "Exactly 1 at full pitch. Only chording lowers it — and that is what \
             cuts the 5th and 7th harmonics. k_p = sin(ν·(y/τ)·90°)."
        }

        #[cfg(feature = "harmonics")]
        (Language::PtBr, "winding_factor") => "Fator de enrolamento",
        #[cfg(feature = "harmonics")]
        (Language::En, "winding_factor") => "Winding factor",
        #[cfg(feature = "harmonics")]
        (Language::PtBr, "winding_factor_hover") => {
            "Fração da FMM ideal (concentrada e de passo pleno) que este \
             enrolamento realmente produz na fundamental."
        }
        #[cfg(feature = "harmonics")]
        (Language::En, "winding_factor_hover") => {
            "The fraction of the ideal (concentrated, full-pitch) MMF this \
             winding actually produces at the fundamental."
        }

        #[cfg(feature = "harmonics")]
        (Language::PtBr, "harmonic_spectrum") => "Espectro harmônico (% da fundamental)",
        #[cfg(feature = "harmonics")]
        (Language::En, "harmonic_spectrum") => "Harmonic spectrum (% of fundamental)",

        (Language::PtBr, "short_pitched") => "Encurtamento de passo",
        (Language::En, "short_pitched") => "Short-pitched coils",

        (Language::PtBr, "short_pitched_needs_layers") => {
            "O encurtamento exige duas camadas elétricas: é o que coloca fases \
             diferentes na mesma ranhura. Com 1 camada o passo não altera a FMM \
             (fator de passo = 1)."
        }
        (Language::En, "short_pitched_needs_layers") => {
            "Chording needs two electrical layers — that is what puts different \
             phases in one slot. With 1 layer the span does not change the MMF \
             (pitch factor = 1)."
        }

        (Language::PtBr, "show_headers") => "Mostrar cabeças de bobina",
        (Language::En, "show_headers") => "Show coil headers",

        (Language::PtBr, "show_vectors") => "Mostrar vetores FMM",
        (Language::En, "show_vectors") => "Show MMF vectors",

        (Language::PtBr, "toggle_headers_hover") => {
            "Alternar visibilidade dos arcos de enrolamento"
        }
        (Language::En, "toggle_headers_hover") => "Toggle visibility of endwinding arcs",

        (Language::PtBr, "valid_config") => "✓ Configuração válida",
        (Language::En, "valid_config") => "✓ Valid configuration",

        (Language::PtBr, "invalid_config") => {
            "⚠ Inválido: ranhuras devem ser divisíveis\n  por 2 × polos × fases"
        }
        (Language::En, "invalid_config") => {
            "⚠ Invalid: grooves must be divisible\n  by 2 × poles × phases"
        }

        (Language::PtBr, "rotate_hint") => "🖱 Botão esquerdo para rotacionar",
        (Language::En, "rotate_hint") => "🖱 Left-click drag to rotate",

        (Language::PtBr, "zoom_hint") => "🖱 Scroll para zoom",
        (Language::En, "zoom_hint") => "🖱 Scroll to zoom",

        (Language::PtBr, "distribution_index") => "Índice de distribuição",
        (Language::En, "distribution_index") => "Distribution index",

        (Language::PtBr, "slot_angle") => "Ângulo entre ranhuras",
        (Language::En, "slot_angle") => "Angle between slots",

        (Language::PtBr, "phase_angle") => "Ângulo entre fases",
        (Language::En, "phase_angle") => "Angle between phases",

        (Language::PtBr, "slots_per_pole_per_phase") => "Ranhuras por polo e por fase",
        (Language::En, "slots_per_pole_per_phase") => "Slots per pole per phase",

        (Language::PtBr, "slots_per_pole") => "Ranhuras por polo",
        (Language::En, "slots_per_pole") => "Slots per pole",

        (Language::PtBr, "total_poles") => "Total de polos",
        (Language::En, "total_poles") => "Total poles",

        (Language::PtBr, "show_mmf_field") => "Mostrar campo FMM",
        (Language::En, "show_mmf_field") => "Show MMF field",

        (Language::PtBr, "toggle_mmf_field_hover") => "Alternar visibilidade do campo FMM",
        (Language::En, "toggle_mmf_field_hover") => "Toggle visibility of MMF field",

        (Language::PtBr, "mmf_gradient_intensity") => "Intensidade do gradiente (γ)",
        (Language::En, "mmf_gradient_intensity") => "Gradient intensity (γ)",

        (Language::PtBr, "mmf_result") => "Resultado",
        (Language::En, "mmf_result") => "Result",

        (Language::PtBr, "mmf_gradient_intensity_hover") => {
            "Controla a concentração do gradiente de campo: valores maiores deixam o campo mais concentrado no eixo magnético"
        }
        (Language::En, "mmf_gradient_intensity_hover") => {
            "Controls gradient sharpness: higher values concentrate the field more towards the magnetic axis"
        }

        (Language::PtBr, "phase_group_label") => "Grupo de fase",
        (Language::En, "phase_group_label") => "Phase group",

        (Language::PtBr, "show_rotor") => "Mostrar rotor",
        (Language::En, "show_rotor") => "Show rotor",

        (Language::PtBr, "show_winding_scheme") => "Mostrar Esquema de Enrolamento",
        (Language::En, "show_winding_scheme") => "Show Winding Scheme",

        (Language::PtBr, "show_winding_scheme_hover") => {
            "Abre uma janela com o diagrama de enrolamento e as formas de onda da FMM"
        }
        (Language::En, "show_winding_scheme_hover") => {
            "Opens a window with the winding diagram and MMF waveforms"
        }

        (Language::PtBr, "winding_scheme_title") => "Esquema de Enrolamento",
        (Language::En, "winding_scheme_title") => "Winding Scheme",

        (Language::PtBr, "winding_diagram") => "Diagrama do Enrolamento",
        (Language::En, "winding_diagram") => "Winding Diagram",

        (Language::PtBr, "winding_function_mmf") => "Função de Enrolamento & FMM",
        (Language::En, "winding_function_mmf") => "Winding Function & MMF",

        (Language::PtBr, "mechanical_angle") => "ângulo mecânico αs (rad)",
        (Language::En, "mechanical_angle") => "mechanical angle αs (rad)",

        (Language::PtBr, "phase_wf") => "fase-{} WF",
        (Language::En, "phase_wf") => "{}-phase WF",

        (Language::PtBr, "total_mmf") => "fmm total",
        (Language::En, "total_mmf") => "total mmf",

        // A missing key is a programming error, not a user-facing failure:
        // fail loudly while developing, but never bring down a shipped build
        // from inside the per-frame UI loop.
        _ => {
            if cfg!(debug_assertions) {
                panic!("Invalid key: {key}");
            }
            "???"
        }
    }
}
