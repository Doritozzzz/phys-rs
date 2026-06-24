//! Spatial components for collision broadphase and world positioning.

use bevy_ecs::prelude::*;

/// Bounding radius for broadphase collision detection [m].
///
/// Represents the radius of the smallest sphere that fully contains
/// the entity's collision geometry. Used by the K-D tree broadphase
/// to generate candidate collision pairs.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct BoundingRadius(pub f64);

/// Orbital predicted orbit for rendering.
///
/// Draws a complete predicted Keplerian orbit from the current state vector.
#[derive(Component, Debug, Clone)]
pub struct OrbitTrail {
    /// Number of points to sample along predicted orbit.
    pub orbit_point_count: usize,
}

impl OrbitTrail {
    pub fn new(orbit_point_count: usize) -> Self {
        Self { orbit_point_count }
    }
}

impl Default for OrbitTrail {
    fn default() -> Self {
        Self { orbit_point_count: 128 }
    }
}
