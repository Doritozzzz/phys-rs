//! Electromagnetic field components (§VII).

use bevy_ecs::prelude::*;
use glam::DVec3;

/// Electric field vector at the entity's position [V m⁻¹].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct ElectricField(pub DVec3);

/// Magnetic field vector at the entity's position [T] (Tesla).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct MagneticField(pub DVec3);

/// Divergence cleaning scalar field ψ [T m s⁻¹].
///
/// Used by the Dedner hyperbolic/parabolic cleaning scheme (§VII.1)
/// to enforce `∇·B = 0`. Evolves alongside `MagneticField` and
/// damps to zero over time.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct DivCleaningPsi(pub f64);

/// Marker component for entities participating in electromagnetic
/// force calculations (Coulomb + Lorentz).
///
/// Entities must also have a `Charge` component with a nonzero value
/// for forces to be applied.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ChargedBody;
