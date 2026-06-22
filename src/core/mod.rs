//! Core foundational types: configuration, time, coordinates, and physical constants.

pub mod config;
pub mod constants;
pub mod coordinates;
pub mod time;

pub use config::{IntegrationMethod, UniverseConfig};
pub use coordinates::{LocalPosition, Sector};
pub use time::{InitialSnapshot, SimulationTime};
