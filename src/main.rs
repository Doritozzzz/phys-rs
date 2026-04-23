//! # phys-rs — Universal Physics Engine
//!
//! Entry point. Sets up the ECS [`World`], inserts resources,
//! and configures the physics [`Schedule`].

mod components;
mod core;
mod gpu;
mod physics;
mod render;

use bevy_ecs::prelude::*;

use crate::core::{IntegrationMethod, SimulationTime, UniverseConfig};

fn main() {
    // --- World Setup ---
    let mut world = World::new();

    let config = UniverseConfig::default();
    let time = SimulationTime::with_dt(
        config.max_dt.min(1.0_f64 / 60.0_f64).max(config.min_dt),
    );

    let integration_method = config.integration_method;
    world.insert_resource(config);
    world.insert_resource(time);

    // --- Schedule Setup ---
    let mut schedule = Schedule::default();

    // Stage 1: Reset accumulators
    schedule.add_systems(
        (physics::reset_forces, physics::reset_torques)
    );

    // Stage 2: Force calculators (gravity, EM, etc.) — added in Phase 4+

    // Stage 3: Compute acceleration from accumulated force
    schedule.add_systems(
        physics::compute_acceleration_system
            .after(physics::reset_forces)
    );

    // Stage 4: Integration (position + velocity update)
    match integration_method {
        IntegrationMethod::SemiImplicitEuler => {
            schedule.add_systems(
                physics::semi_implicit_euler_system
                    .after(physics::compute_acceleration_system)
            );
        }
        IntegrationMethod::VelocityVerlet => {
            schedule.add_systems(
                physics::velocity_verlet_position_system
                    .after(physics::compute_acceleration_system)
            );
            // NOTE: Verlet velocity step needs a second force evaluation.
            // For now, we approximate with single-evaluation Verlet.
            schedule.add_systems(
                physics::velocity_verlet_velocity_system
                    .after(physics::velocity_verlet_position_system)
            );
        }
        IntegrationMethod::RungeKutta4 => {
            schedule.add_systems(
                physics::rk4_system
                    .after(physics::compute_acceleration_system)
            );
        }
    }

    // Stage 5: Sector boundary normalization
    schedule.add_systems(
        physics::sector_boundary_system
            .after(physics::compute_acceleration_system)
    );

    // Stage 6: Momentum tracking
    schedule.add_systems(
        physics::update_momentum_system
            .after(physics::compute_acceleration_system)
    );

    // Stage 7: Rotational integration
    schedule.add_systems(
        physics::rotational_integration_system
            .after(physics::compute_acceleration_system)
    );

    // --- Main Loop ---
    // For now, run a single tick to verify setup.
    // The full game loop with winit will be added in Sandbox Phase S1.
    schedule.run(&mut world);

    println!("phys-rs engine initialized. Schedule executed successfully.");
}
