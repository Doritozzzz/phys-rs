//! Dynamic quantities: mass, velocity, acceleration, force, momentum.
//!
//! These components represent the kinematic and kinetic state of entities.
//! All values use `f64` precision and SI units.

use bevy_ecs::prelude::*;
use glam::DVec3;

/// Inertial mass of an entity [kg].
///
/// Must be strictly positive for physics entities.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Mass(pub f64);

/// Linear velocity [m s⁻¹].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Velocity(pub DVec3);

/// Linear acceleration [m s⁻²].
///
/// Computed from accumulated forces: `a = F / m`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Acceleration(pub DVec3);

/// Accumulated force vector [N].
///
/// Reset to zero at the start of each tick by the force accumulator system.
/// Force calculator systems (gravity, EM, etc.) add their contributions.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Force(pub DVec3);

/// Previous tick's acceleration [m s⁻²].
///
/// Required by the Velocity Verlet integrator (PHYSICS_MASTER_INDEX §I.2)
/// for the half-step velocity update.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct PreviousAcceleration(pub DVec3);

/// Linear momentum [kg m s⁻¹].
///
/// `p = m * v`. Updated after integration for conservation tracking.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct LinearMomentum(pub DVec3);
