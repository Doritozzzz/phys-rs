//! All ECS Component definitions.
//!
//! Components are pure data — no logic, no methods with side effects.
//! Each module groups related physical quantities.

pub mod collisions;
pub mod dynamics;
pub mod electromagnetic;
pub mod identifiers;
pub mod material;
pub mod rotational;
pub mod spatial;
pub mod sph;
pub mod thermal;

pub use dynamics::{
    Acceleration, Force, LinearMomentum, LorentzFactor, DopplerShift, RelativisticMomentum,
    Mass, PreviousAcceleration, Velocity,
};
pub use electromagnetic::{ChargedBody, DivCleaningPsi, ElectricField, MagneticField};
pub use identifiers::{BodyType, EntityName};
pub use material::{Charge, Density, Luminosity, Opacity, Temperature};
pub use rotational::{AngularVelocity, InertiaTensor, Orientation, Torque};
pub use spatial::BoundingRadius;
pub use sph::{FluidParticle, Pressure, SmoothedDensity, SmoothingRadius};
pub use thermal::{HeatCapacity, InternalEnergy, ThermalConductivity};
