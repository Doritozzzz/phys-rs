//! Rotational dynamics components.
//!
//! Angular velocity, orientation (quaternion), torque, and inertia tensor.
//! Uses `DQuat` from `glam` for gimbal-lock-free rotation (§II.3)
//! and `Matrix3<f64>` from `nalgebra` for inertia tensors (§II.2).

use bevy_ecs::prelude::*;
use glam::{DQuat, DVec3};

/// Angular velocity vector [rad s⁻¹].
///
/// The direction indicates the axis of rotation (right-hand rule),
/// the magnitude indicates the rotation speed.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct AngularVelocity(pub DVec3);

/// Orientation quaternion (unit quaternion).
///
/// Represents the body's rotation relative to the world frame.
/// Must remain normalized; integrators re-normalize after each step.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Orientation(pub DQuat);

impl Default for Orientation {
    fn default() -> Self {
        Self(DQuat::IDENTITY)
    }
}

/// Accumulated torque vector [N m].
///
/// Reset to zero at the start of each tick. Force systems and
/// collision contacts contribute torque here.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Torque(pub DVec3);

/// Moment of inertia tensor [kg m²].
///
/// 3×3 symmetric matrix in body-local frame. Stored as a flat
/// `[f64; 9]` in row-major order for `nalgebra::Matrix3` compatibility.
///
/// For a solid sphere of mass `m` and radius `r`:
/// `I = diag(2/5 mr², 2/5 mr², 2/5 mr²)` (PHYSICS_MASTER_INDEX §II.2)
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct InertiaTensor(pub [f64; 9]);

impl Default for InertiaTensor {
    /// Default: identity matrix (unit inertia).
    fn default() -> Self {
        Self([
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ])
    }
}

impl InertiaTensor {
    /// Create a diagonal inertia tensor (principal axes aligned with world).
    pub fn diagonal(ix: f64, iy: f64, iz: f64) -> Self {
        Self([
            ix,  0.0, 0.0,
            0.0, iy,  0.0,
            0.0, 0.0, iz,
        ])
    }

    /// Create the inertia tensor for a solid sphere of mass `m` and radius `r`.
    ///
    /// `I = 2/5 mr²` on all principal axes (PHYSICS_MASTER_INDEX §II.2).
    pub fn solid_sphere(mass: f64, radius: f64) -> Self {
        let i = 0.4_f64 * mass * radius * radius;
        Self::diagonal(i, i, i)
    }
}
