pub mod actions;
pub mod components;

use bevy_ecs::prelude::Entity;
use glam::DVec3;
use crate::components::BodyType;
use self::actions::UiAction;

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

/// Dibuja la UI y devuelve las acciones que el usuario realizó este frame.
pub fn draw_ui(
    ctx: &egui::Context,
    entity_list: &[(Entity, String)],
    selected_entity: Option<Entity>,
    hud_data: Option<&EntityHudData>,
) -> Vec<UiAction> {
    let mut actions = Vec::new();

    components::inspector::draw(ctx, entity_list, selected_entity, &mut actions);

    if let (Some(entity), Some(data)) = (selected_entity, hud_data) {
        components::hud::draw(ctx, entity, data, &mut actions);
    }

    actions
}
