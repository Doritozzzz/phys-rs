//! Spatial components for collision broadphase and world positioning.

use bevy_ecs::prelude::*;

/// Bounding radius for broadphase collision detection [m].
///
/// Represents the radius of the smallest sphere that fully contains
/// the entity's collision geometry. Used by the K-D tree broadphase
/// to generate candidate collision pairs.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct BoundingRadius(pub f64);
