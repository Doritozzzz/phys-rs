pub mod actions;
pub mod components;

use bevy_ecs::prelude::Entity;
use glam::DVec3;
use crate::components::BodyType;
use self::actions::UiAction;

use crate::physics::energy::EnergyMonitor;

/// Datos que la lógica de renderizado principal recaba de ECS
/// para mostrar en la interfaz de detalle.
pub struct EntityHudData {
    pub name: String,
    pub mass: Option<f64>,
    pub velocity: Option<DVec3>,
    pub position: Option<DVec3>,
    pub temperature: Option<f64>,
    pub body_type: Option<BodyType>,
}

/// Estado persistente de la UI
#[derive(Default)]
pub struct UiState {
    pub energy_history: std::collections::VecDeque<(f64, f64, f64, f64)>, // (time, kinetic, potential, total)
    pub current_time: f64,
    // Console state
    pub console_input: String,
    pub console_history: Vec<String>,
    pub show_console: bool,
    // Spawner state
    pub spawner_mass_log: f32,
    pub spawner_vel: [f32; 3],
    pub spawner_temp: f32,
    pub spawner_type: BodyType,
    pub show_spawner: bool,
    // Profiler state
    pub show_profiler: bool,
    // Graph state
    pub show_energy_graph: bool,
}

/// Dibuja la UI y devuelve las acciones que el usuario realizó este frame.
pub fn draw_ui(
    ctx: &egui::Context,
    state: &mut UiState,
    entity_list: &[(Entity, String)],
    selected_entity: Option<Entity>,
    hud_data: Option<&EntityHudData>,
    energy_data: Option<&EnergyMonitor>,
    diagnostics: Option<&crate::core::diagnostics::Diagnostics>,
) -> Vec<UiAction> {
    let mut actions = Vec::new();

    components::inspector::draw(ctx, entity_list, selected_entity, &mut actions);

    if let (Some(entity), Some(data)) = (selected_entity, hud_data) {
        components::hud::draw(ctx, entity, data, &mut actions);
    }
    
    // Top menu bar to toggle windows
    egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut state.show_energy_graph, "Energy Graph");
            ui.toggle_value(&mut state.show_spawner, "Spawner");
            ui.toggle_value(&mut state.show_console, "Console");
            ui.toggle_value(&mut state.show_profiler, "Profiler");
        });
    });

    if let Some(energy) = energy_data {
        components::energy_graph::draw(ctx, state, energy);
    }
    
    components::spawner::draw(ctx, state, &mut actions);
    components::console::draw(ctx, state, &mut actions);
    components::serialization::draw(ctx, state, &mut actions);
    components::time_control::draw(ctx, state, &mut actions);
    components::profiler::draw(ctx, state, diagnostics);

    actions
}
