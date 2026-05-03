//! Smoothed Particle Hydrodynamics (SPH) components (§V).

use bevy_ecs::prelude::*;

/// Smoothed density estimate at the particle's position [kg m⁻³].
///
/// Computed via kernel interpolation: `ρᵢ = Σⱼ mⱼ W(rᵢ-rⱼ, h)`
/// (PHYSICS_MASTER_INDEX §V.1). Includes self-contribution.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct SmoothedDensity(pub f64);

/// Pressure at the particle's position [Pa].
///
/// Derived from the equation of state (§V.2):
/// ideal gas `P = k(ρ - ρ₀)` or Tait equation.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Pressure(pub f64);

/// Smoothing radius (support radius) for SPH kernel [m].
///
/// Determines the neighborhood size for particle interactions.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct SmoothingRadius(pub f64);

/// Marker component identifying an entity as an SPH fluid particle.
///
/// Attach this to any entity that should participate in SPH
/// density estimation, pressure, and viscosity calculations.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FluidParticle;
