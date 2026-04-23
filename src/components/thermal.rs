//! Thermal components for heat transfer and thermodynamics (§VI).

use bevy_ecs::prelude::*;

/// Internal (thermal) energy [J].
///
/// For an ideal monatomic gas: `U = 3/2 N k_B T` (PHYSICS_MASTER_INDEX §VI.1).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct InternalEnergy(pub f64);

/// Specific heat capacity at constant volume [J kg⁻¹ K⁻¹].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct HeatCapacity(pub f64);

/// Thermal conductivity [W m⁻¹ K⁻¹].
///
/// Used in Fourier's law: `q = -k ∇T` (PHYSICS_MASTER_INDEX §VI.2).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct ThermalConductivity(pub f64);
