//! Dynamic quantities: mass, velocity, acceleration, force, momentum.
//!
//! These components represent the kinematic and kinetic state of entities.
//! All values use `f64` precision and SI units.

use bevy_ecs::prelude::*;
use glam::DVec3;

/// Inertial mass of an entity [kg].
///
/// Must be strictly positive for physics entities.
#[derive(Component, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mass(pub f64);

/// Linear velocity [m s⁻¹].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Velocity(pub DVec3);

/// Linear acceleration [m s⁻²].
///
/// Computed from accumulated forces: `a = F / m`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Acceleration(pub DVec3);

/// Accumulated force vector [N].
///
/// Reset to zero at the start of each tick by the force accumulator system.
/// Force calculator systems (gravity, EM, etc.) add their contributions.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Force(pub DVec3);

/// Previous tick's acceleration [m s⁻²].
///
/// Required by the Velocity Verlet integrator (PHYSICS_MASTER_INDEX §I.2)
/// for the half-step velocity update.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct PreviousAcceleration(pub DVec3);

/// Linear momentum [kg m s⁻¹].
///
/// `p = m * v`. Updated after integration for conservation tracking.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearMomentum(pub DVec3);

/// Lorentz factor γ [dimensionless] (PHYSICS_MASTER_INDEX §VIII).
///
/// `γ = 1 / √(1 - v²/c²)`. Cached per entity each tick.
/// Equals 1.0 for non-relativistic bodies (v << c).
/// Opt-in: only entities with this component participate in SR corrections.
#[derive(Component, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LorentzFactor(pub f64);

impl Default for LorentzFactor {
    fn default() -> Self {
        Self(1.0_f64)
    }
}

/// Relativistic Doppler shift ratio [dimensionless] (PHYSICS_MASTER_INDEX §VIII).
///
/// `f_obs / f_emit = √((1 - v_r/c) / (1 + v_r/c))` where v_r is
/// the radial velocity toward the observer. Stored for rendering use.
/// - Values < 1.0 → redshift (receding)
/// - Values > 1.0 → blueshift (approaching)
#[derive(Component, Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DopplerShift(pub f64);

impl Default for DopplerShift {
    fn default() -> Self {
        Self(1.0_f64)
    }
}

/// Relativistic momentum [kg m s⁻¹] (PHYSICS_MASTER_INDEX §VIII).
///
/// `p = γ m₀ v`. Diverges as v → c. Tracked alongside classical
/// `LinearMomentum` for relativistic bodies.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct RelativisticMomentum(pub DVec3);
