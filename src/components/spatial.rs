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

/// Mode for orbit visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbitMode {
    /// Draw trail of past positions.
    Trail,
    /// Draw complete predicted Keplerian orbit from current state.
    Predicted,
}

impl Default for OrbitMode {
    fn default() -> Self {
        Self::Predicted
    }
}

/// Orbital trail / predicted orbit for rendering.
///
/// In `Trail` mode, stores the last N world positions.
/// In `Predicted` mode, draws a full Keplerian orbit from state vector.
#[derive(Component, Debug, Clone)]
pub struct OrbitTrail {
    /// Queue of sector+local positions (newest first). Used only in Trail mode.
    pub history: VecDeque<(Sector, LocalPosition)>,
    /// Maximum number of trail points to retain.
    pub max_points: usize,
    /// Visualization mode.
    pub mode: OrbitMode,
    /// Number of points to sample along predicted orbit.
    pub orbit_point_count: usize,
}

impl OrbitTrail {
    pub fn new(max_points: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_points.min(4096)),
            max_points,
            mode: OrbitMode::Predicted,
            orbit_point_count: 128,
        }
    }
}

impl Default for OrbitTrail {
    fn default() -> Self {
        Self::new(256)
    }
}
