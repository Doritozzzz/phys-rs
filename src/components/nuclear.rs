//! Nuclear astrophysics components.

use bevy_ecs::prelude::*;

/// Tracks the core state of a stellar body for nuclear reactions.
#[derive(Component, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StellarCore {
    /// Core temperature in Kelvin.
    pub temperature: f64,
    /// Core density in kg/m³.
    pub density: f64,
    /// Mass fraction of Hydrogen (X).
    pub hydrogen_fraction: f64,
    /// Mass fraction of Helium (Y).
    pub helium_fraction: f64,
    /// Mass fraction of heavier elements (Z).
    pub metallicity: f64,
}

impl Default for StellarCore {
    fn default() -> Self {
        Self {
            temperature: 1.5e7_f64, // Typical solar core T
            density: 1.5e5_f64,     // Typical solar core density
            hydrogen_fraction: 0.73_f64,
            helium_fraction: 0.25_f64,
            metallicity: 0.02_f64,
        }
    }
}

/// The current evolutionary state of a star.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StellarState {
    /// Hydrogen burning core.
    MainSequence,
    /// Hydrogen shell burning / Helium core burning.
    RedGiant,
    /// Degenerate electron core remnant.
    WhiteDwarf,
    /// Degenerate neutron core remnant.
    NeutronStar,
    /// Singularity / event horizon.
    BlackHole,
}

/// Event triggered when a stellar core collapses (e.g., exceeds Chandrasekhar limit).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupernovaEvent {
    /// The entity that collapsed.
    pub entity: Entity,
    /// The final mass of the core before collapse.
    pub core_mass: f64,
    /// The resulting remnant type (NeutronStar or BlackHole).
    pub remnant_type: StellarState,
}

#[derive(Resource, Default, serde::Serialize, serde::Deserialize)]
pub struct SupernovaEvents(pub Vec<SupernovaEvent>);
