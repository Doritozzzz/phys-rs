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
    world.insert_resource(physics::EnergyMonitor::default());
    world.insert_resource(crate::components::collisions::CollisionEvents::default());
    world.insert_resource(physics::CandidatePairs(Vec::new()));
    world.insert_resource(physics::KdTreeBroadphase::default());
    world.insert_resource(physics::TaitEquationConfig::default());
    world.insert_resource(physics::SphViscosityConfig::default());
    world.insert_resource(physics::XsphConfig::default());
    world.insert_resource(physics::SphBoundaryConfig::default());
    world.insert_resource(physics::RadiationConfig::default());
    world.insert_resource(physics::ThermalConductionConfig::default());

    // --- Schedule Setup ---
    let mut schedule = Schedule::default();

    // Stage 1: Reset accumulators
    schedule.add_systems(
        (physics::reset_forces, physics::reset_torques)
    );

    // Stage 1.5: Spatial partitioning (broadphase)
    // Runs early so gravity tree, SPH, and collisions can all use the
    // K-D tree neighbor data from a single build per tick.
    schedule.add_systems(
        physics::build_broadphase_system
            .after(physics::reset_forces)
    );
    schedule.add_systems(
        physics::broadphase_query_system
            .after(physics::build_broadphase_system)
    );

    // Stage 2: Gravitational force calculator
    schedule.add_systems(
        physics::brute_force_gravity_system
            .after(physics::broadphase_query_system)
    );

    // Stage 2.5: SPH fluid dynamics pipeline (§V)
    //   density → equation of state → pressure force → viscosity
    schedule.add_systems(
        physics::sph_density_system
            .after(physics::broadphase_query_system)
    );
    schedule.add_systems(
        physics::sph_eos_system
            .after(physics::sph_density_system)
    );
    schedule.add_systems(
        physics::sph_pressure_force_system
            .after(physics::sph_eos_system)
    );
    schedule.add_systems(
        physics::sph_viscosity_system
            .after(physics::sph_eos_system)
    );

    // Stage 2.7: Boundary repulsion for domain-confined SPH
    schedule.add_systems(
        physics::sph_boundary_system
            .after(physics::sph_viscosity_system)
    );

    // Stage 3: Compute acceleration from ALL accumulated forces
    schedule.add_systems(
        physics::compute_acceleration_system
            .after(physics::brute_force_gravity_system)
            .after(physics::sph_pressure_force_system)
            .after(physics::sph_viscosity_system)
            .after(physics::sph_boundary_system)
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

    // Stage 4.5: XSPH velocity smoothing (post-integration, SPH only)
    schedule.add_systems(
        physics::sph_xsph_system
            .after(physics::velocity_verlet_velocity_system)
    );

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

    // Stage 7: Rotational dynamics
    schedule.add_systems(
        physics::compute_angular_acceleration_system
            .after(physics::reset_torques)
            .after(physics::update_momentum_system) 
    );
    schedule.add_systems(
        physics::damping_system
            .after(physics::compute_acceleration_system)
    );
    schedule.add_systems(
        physics::rotational_integration_system
            .after(physics::compute_angular_acceleration_system)
            .after(physics::damping_system)
            .after(physics::velocity_verlet_velocity_system)
    );

    // Stage 7.5: Narrowphase collisions & impulse resolution
    // (Broadphase already ran in Stage 1.5)
    schedule.add_systems(
        physics::narrow_phase_system
            .after(physics::rotational_integration_system)
            .after(physics::sector_boundary_system)
    );
    schedule.add_systems(
        physics::collision_impulse_system
            .after(physics::narrow_phase_system)
    );

    // Stage 8: Energy monitoring
    schedule.add_systems(
        (
            physics::compute_kinetic_energy_system,
            physics::compute_potential_energy_system,
        ).after(physics::update_momentum_system)
    );
    schedule.add_systems(
        physics::energy_drift_monitor_system
            .after(physics::compute_kinetic_energy_system)
            .after(physics::compute_potential_energy_system)
    );

    // Stage 9: Thermodynamics (§VI)
    //   sync T from U → conduction → radiation → sync U from T
    schedule.add_systems(
        physics::sync_temperature_system
            .after(physics::collision_impulse_system)
    );
    schedule.add_systems(
        physics::heat_conduction_system
            .after(physics::sync_temperature_system)
    );
    schedule.add_systems(
        physics::radiative_cooling_system
            .after(physics::heat_conduction_system)
    );
    schedule.add_systems(
        physics::sync_internal_energy_system
            .after(physics::radiative_cooling_system)
    );

    // --- Main Loop ---
    // For now, run a single tick to verify setup.
    // The full game loop with winit will be added in Sandbox Phase S1.
    schedule.run(&mut world);

    println!("phys-rs engine initialized. Schedule executed successfully.");
}
