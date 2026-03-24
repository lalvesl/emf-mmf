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

        (Language::PtBr, "rotate_hint") => "🖱 Botão direito para rotacionar",
        (Language::En, "rotate_hint") => "🖱 Right-click drag to rotate",

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

        _ => panic!("Invalid key: {}", key),
    }
}
