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

        (Language::PtBr, "short_pitched") => "Encurtamento de passo",
        (Language::En, "short_pitched") => "Short-pitched coils",

        (Language::PtBr, "show_headers") => "Mostrar cabeças de bobina",
        (Language::En, "show_headers") => "Show coil headers",

        (Language::PtBr, "show_vectors") => "Mostrar vetores FMM",
        (Language::En, "show_vectors") => "Show MMF vectors",

        (Language::PtBr, "show_fields") => "Mostrar campos magnéticos",
        (Language::En, "show_fields") => "Show magnetic fields",

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

        _ => panic!("Invalid key: {}", key),
    }
}
