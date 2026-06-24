use bevy_ecs::prelude::Entity;

/// Acciones emitidas por la UI para ser gestionadas por la lógica de renderizado principal.
pub enum UiAction {
    /// Solicitud para cambiar la entidad seleccionada actualmente.
    SelectEntity(Entity),
    /// Solicitud para limpiar la selección actual.
    ClearSelection,
    /// Solicitud para mover la cámara hacia una entidad concreta.
    FocusCamera(Entity),
    /// Solicitud para crear una nueva entidad.
    SpawnEntity {
        mass: f64,
        position: glam::DVec3,
        velocity: glam::DVec3,
        temperature: f64,
        body_type: crate::components::BodyType,
    },
    /// Comando ingresado en la consola.
    ExecuteCommand(String),
    /// Guardado rápido del estado de la simulación.
    QuickSave,
    /// Carga rápida del estado de la simulación.
    QuickLoad,
    /// Cambiar la velocidad de simulación.
    SetTimeScale(f64),
    /// Alternar pausa de la simulación.
    TogglePause,
    /// Avanzar un solo tick.
    StepTick,
}
