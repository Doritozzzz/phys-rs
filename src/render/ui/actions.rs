use bevy_ecs::prelude::Entity;

/// Acciones emitidas por la UI para ser gestionadas por la lógica de renderizado principal.
pub enum UiAction {
    /// Solicitud para cambiar la entidad seleccionada actualmente.
    SelectEntity(Entity),
    /// Solicitud para limpiar la selección actual.
    ClearSelection,
    /// Solicitud para mover la cámara hacia una entidad concreta.
    FocusCamera(Entity),
}
