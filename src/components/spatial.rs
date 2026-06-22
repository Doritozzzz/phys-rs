//! Spatial components for collision broadphase and world positioning.

use std::collections::VecDeque;
use bevy_ecs::prelude::*;
use crate::core::coordinates::{LocalPosition, Sector};

/// Bounding radius for broadphase collision detection [m].
///
/// Represents the radius of the smallest sphere that fully contains
/// the entity's collision geometry. Used by the K-D tree broadphase
/// to generate candidate collision pairs.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct BoundingRadius(pub f64);

/// Orbital trail history for rendering.
///
/// Stores the last N world positions for drawing orbit trails.
/// Updated by the trail collection system each frame.
#[derive(Component, Debug, Clone)]
pub struct OrbitTrail {
    /// Queue of sector+local positions (newest first).
    pub history: VecDeque<(Sector, LocalPosition)>,
    /// Maximum number of points to retain.
    pub max_points: usize,
}

impl OrbitTrail {
    pub fn new(max_points: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_points.min(4096)),
            max_points,
        }
    }
}
