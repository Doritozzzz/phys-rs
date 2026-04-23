//! Force and torque accumulator reset system.
//!
//! Runs at the START of each physics tick to zero out accumulated
//! forces and torques before force-calculator systems add their contributions.

use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::{Force, Torque};

/// Resets all `Force` components to zero.
///
/// Must run before any force calculator system (gravity, EM, SPH, etc.)
/// in the schedule to ensure forces are not double-counted across ticks.
pub fn reset_forces(mut query: Query<&mut Force>) {
    for mut force in &mut query {
        force.0 = DVec3::ZERO;
    }
}

/// Resets all `Torque` components to zero.
///
/// Must run before any torque calculator system.
pub fn reset_torques(mut query: Query<&mut Torque>) {
    for mut torque in &mut query {
        torque.0 = DVec3::ZERO;
    }
}
