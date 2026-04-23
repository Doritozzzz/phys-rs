//! Physics systems — all simulation logic lives here.
//!
//! Systems are plain functions operating on ECS components via queries.
//! No data definitions here; those belong in `components/`.

pub mod forces;
pub mod integrators;
pub mod origin;

pub use forces::{reset_forces, reset_torques};
pub use integrators::{
    compute_acceleration_system, rk4_system, rotational_integration_system,
    semi_implicit_euler_system, update_momentum_system, velocity_verlet_position_system,
    velocity_verlet_velocity_system,
};
pub use origin::sector_boundary_system;
