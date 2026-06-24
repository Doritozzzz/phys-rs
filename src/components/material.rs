//! Material properties: temperature, charge, density, luminosity, opacity.

use bevy_ecs::prelude::*;

/// Temperature [K] (Kelvin).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Temperature(pub f64);

/// Electric charge [C] (Coulombs).
///
/// Positive or negative. Zero for electrically neutral bodies.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Charge(pub f64);

/// Mass density [kg m⁻³].
///
/// For rigid bodies, this is the bulk density.
/// For SPH particles, this is the smoothed field density.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Density(pub f64);

/// Bolometric luminosity [W] (Watts).
///
/// Total electromagnetic power emitted by the body.
/// For stars, derived from Stefan-Boltzmann: `L = 4πR²σT⁴`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Luminosity(pub f64);

/// Opacity [m² kg⁻¹].
///
/// Measure of how opaque the material is to radiation.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Opacity(pub f64);
