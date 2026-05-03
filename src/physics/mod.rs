//! Physics systems — all simulation logic lives here.
//!
//! Systems are plain functions operating on ECS components via queries.
//! No data definitions here; those belong in `components/`.

pub mod broadphase;
pub mod collisions;
pub mod energy;
pub mod forces;
pub mod gravity;
pub mod gravity_tree;
pub mod integrators;
pub mod orbital;
pub mod origin;
pub mod rigid_body;
pub mod sph;

pub use broadphase::{
    broadphase_query_system, build_broadphase_system, CandidatePairs, KdTreeBroadphase,
};
pub use collisions::{collision_impulse_system, narrow_phase_system};
pub use energy::{
    compute_kinetic_energy_system, compute_potential_energy_system, energy_drift_monitor_system,
    EnergyMonitor,
};
pub use forces::{reset_forces, reset_torques};
pub use gravity::brute_force_gravity_system;
pub use gravity_tree::barnes_hut_gravity_system;
pub use integrators::{
    compute_acceleration_system, rk4_system, rotational_integration_system,
    semi_implicit_euler_system, update_momentum_system, velocity_verlet_position_system,
    velocity_verlet_velocity_system,
};
pub use orbital::{escape_velocity, roche_limit, schwarzschild_radius, vis_viva_velocity};
pub use origin::sector_boundary_system;
pub use rigid_body::*;
pub use sph::{
    sph_density_system, sph_eos_system, sph_pressure_force_system, sph_viscosity_system,
    sph_xsph_system, SphViscosityConfig, TaitEquationConfig, XsphConfig,
};

