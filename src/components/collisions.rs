use bevy_ecs::prelude::*;
use glam::DVec3;
use parry3d_f64::shape::SharedShape;

/// Shared geometry shape for collision narrowphase.
///
/// Wraps `parry3d_f64::shape::SharedShape` which can be a Sphere, Cuboid, Capsule, etc.
#[derive(Component, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollisionShape(pub SharedShape);

/// Coefficient of Restitution for collision impulse.
///
/// 0.0 = perfectly inelastic (bodies stick together, all kinetic energy lost to deformation/heat).
/// 1.0 = perfectly elastic (no loss of kinetic energy).
#[derive(Component, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoefficientOfRestitution(pub f64);

impl Default for CoefficientOfRestitution {
    fn default() -> Self {
        Self(0.5) // Default to partially elastic
    }
}

/// Friction coefficient for tangential impulses.
///
/// Maps to standard kinetic friction `mu_k`.
#[derive(Component, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FrictionCoefficient(pub f64);

impl Default for FrictionCoefficient {
    fn default() -> Self {
        Self(0.3) // Default kinetic friction
    }
}

/// Event emitted when a collision occurs between two bodies.
#[derive(Debug, Clone)]
pub struct CollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub normal: DVec3,
    pub point: DVec3,
    pub impulse_magnitude: f64,
}

#[derive(Resource, Default)]
pub struct CollisionEvents(pub Vec<CollisionEvent>);
