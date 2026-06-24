//! Entity identification and classification components.

use bevy_ecs::prelude::*;

/// Human-readable name for an entity.
#[derive(Component, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntityName(pub String);

/// Classification of a physics body.
///
/// Used for filtering queries, rendering decisions, and
/// scenario-specific behavior.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum BodyType {
    /// Main-sequence or evolved star.
    Star,
    /// Planetary body (rocky or gas giant).
    #[default]
    Planet,
    /// Natural satellite.
    Moon,
    /// Small rocky/metallic body.
    Asteroid,
    /// Generic point particle (no rotation).
    Particle,
    /// SPH fluid element.
    FluidParticle,
    /// Stellar remnant: White Dwarf.
    WhiteDwarf,
    /// Stellar remnant: Neutron Star.
    NeutronStar,
    /// Stellar remnant: Black Hole.
    BlackHole,
}
