use bevy_ecs::prelude::Component;

/// Marker component for entities that gravitationally lens background light.
///
/// Reuses `Mass` and `BoundingRadius` from the same entity for lens parameters.
/// Zero fields — pure marker.
#[derive(Component, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GravitationalLens;
