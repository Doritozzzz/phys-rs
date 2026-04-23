//! Universe configuration resource.
//!
//! Central configuration for all tunable simulation parameters.
//! Inserted as a [`Resource`] into the ECS [`World`].

use bevy_ecs::prelude::*;

/// Method of numerical integration used by the simulation.
///
/// See PHYSICS_MASTER_INDEX §I for equations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationMethod {
    /// Semi-Implicit Euler — symplectic, first-order (§I.1).
    SemiImplicitEuler,
    /// Velocity Verlet — symplectic, second-order, energy-conserving (§I.2).
    VelocityVerlet,
    /// Runge-Kutta 4th Order — non-symplectic, fourth-order accuracy (§I.3).
    RungeKutta4,
}

/// Global simulation configuration resource.
///
/// Contains all tunable parameters for the physics engine.
/// Modify these to change simulation behavior without recompiling.
#[derive(Resource, Debug, Clone)]
pub struct UniverseConfig {
    /// Gravitational constant G [m³ kg⁻¹ s⁻²].
    pub gravitational_constant: f64,

    /// Softening parameter ε [m] for force singularity prevention.
    /// Used in denominators: `r² + ε²` instead of bare `r²`.
    pub softening_epsilon: f64,

    /// Maximum allowed timestep [s]. Safety clamp.
    pub max_dt: f64,

    /// Minimum allowed timestep [s]. Safety clamp.
    pub min_dt: f64,

    /// Barnes-Hut opening angle θ [dimensionless].
    /// Lower values = more accurate but slower.
    pub barnes_hut_theta: f64,

    /// Active numerical integration method.
    pub integration_method: IntegrationMethod,

    /// Sector size [m]. Side length of each cubic sector.
    pub sector_size: f64,

    /// Origin shift threshold [m]. When a body's `LocalPosition`
    /// magnitude exceeds this, the origin is re-centered.
    pub origin_shift_threshold: f64,
}

impl Default for UniverseConfig {
    fn default() -> Self {
        let sector_size = 1e12_f64;
        Self {
            gravitational_constant: super::constants::G,
            softening_epsilon: 1e-4_f64,
            max_dt: 1e-1_f64,
            min_dt: 1e-6_f64,
            barnes_hut_theta: 0.5_f64,
            integration_method: IntegrationMethod::VelocityVerlet,
            sector_size,
            origin_shift_threshold: sector_size * 0.4_f64,
        }
    }
}
