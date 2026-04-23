//! Electromagnetic field components (§VII).

use bevy_ecs::prelude::*;
use glam::DVec3;

/// Electric field vector at the entity's position [V m⁻¹].
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct ElectricField(pub DVec3);

/// Magnetic field vector at the entity's position [T] (Tesla).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct MagneticField(pub DVec3);
