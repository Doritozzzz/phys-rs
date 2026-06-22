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
    world.insert_resource(physics::MhdConfig::default());
    world.insert_resource(physics::NfwHaloConfig::default());

    // --- Schedule Setup ---
    //
    // IMPORTANT: bevy_ecs 0.18 forbids `.after(SystemTypeSet(fn))` when the
    // same function appears more than once in the schedule (e.g., Velocity
    // Verlet's second force evaluation duplicates the first-pass systems).
    // We split into ≤21-element `.chain()` groups and use `.after()` only
    // with systems that appear EXACTLY ONCE in each branch.
    //
    // The entire pipeline lives inside the match block so each integration
    // method produces exactly one schedule topology.
    let mut schedule = Schedule::default();

    match integration_method {
        IntegrationMethod::SemiImplicitEuler => {
            // Group 1: force evaluation (16 systems)
            schedule.add_systems((
                physics::reset_forces,
                physics::reset_torques,
                physics::build_broadphase_system,
                physics::broadphase_query_system,
                physics::brute_force_gravity_system,
                physics::nfw_gravity_system,
                physics::coulomb_force_system,
                physics::lorentz_force_system,
                physics::geodesic_correction_system,
                physics::sph_density_system,
                physics::sph_eos_system,
                physics::sph_pressure_force_system,
                physics::sph_viscosity_system,
                physics::sph_boundary_system,
                physics::compute_acceleration_system,
            ).chain());

            // Group 2: integration + post-integration (8 systems, after group 1)
            schedule.add_systems((
                physics::semi_implicit_euler_system,
                physics::sph_xsph_system,
                physics::mhd_induction_system,
                physics::divergence_cleaning_system,
                physics::compute_lorentz_factor_system,
                physics::relativistic_momentum_system,
                physics::compute_doppler_shift_system,
                physics::sector_boundary_system,
            ).chain().after(physics::compute_acceleration_system));

            // Group 3: rest of pipeline (14 systems, sector_boundary unique in branch)
            schedule.add_systems((
                physics::update_momentum_system,
                physics::compute_angular_acceleration_system,
                physics::damping_system,
                physics::rotational_integration_system,
                physics::lense_thirring_system,
                physics::narrow_phase_system,
                physics::collision_impulse_system,
                physics::compute_kinetic_energy_system,
                physics::compute_potential_energy_system,
                physics::energy_drift_monitor_system,
                physics::sync_temperature_system,
                physics::heat_conduction_system,
                physics::radiative_cooling_system,
                physics::sync_internal_energy_system,
            ).chain().after(physics::sector_boundary_system));
        }
        IntegrationMethod::VelocityVerlet => {
            // Pass 1 — force evaluation + Verlet position step (16 systems)
            schedule.add_systems((
                physics::reset_forces,
                physics::reset_torques,
                physics::build_broadphase_system,
                physics::broadphase_query_system,
                physics::brute_force_gravity_system,
                physics::nfw_gravity_system,
                physics::coulomb_force_system,
                physics::lorentz_force_system,
                physics::geodesic_correction_system,
                physics::sph_density_system,
                physics::sph_eos_system,
                physics::sph_pressure_force_system,
                physics::sph_viscosity_system,
                physics::sph_boundary_system,
                physics::compute_acceleration_system,
                physics::velocity_verlet_position_system,
            ).chain());

            // Pass 2a — SECOND force evaluation + Verlet velocity step (17 systems)
            // velocity_verlet_position_system appears only in Pass 1 → unambiguous
            schedule.add_systems((
                physics::reset_forces,
                physics::reset_torques,
                physics::build_broadphase_system,
                physics::broadphase_query_system,
                physics::brute_force_gravity_system,
                physics::nfw_gravity_system,
                physics::coulomb_force_system,
                physics::lorentz_force_system,
                physics::geodesic_correction_system,
                physics::sph_density_system,
                physics::sph_eos_system,
                physics::sph_pressure_force_system,
                physics::sph_viscosity_system,
                physics::sph_boundary_system,
                physics::compute_acceleration_system,
                physics::velocity_verlet_velocity_system,
            ).chain().after(physics::velocity_verlet_position_system));

            // Pass 2b — post-integration + momentum + rotational + collisions (14 systems)
            schedule.add_systems((
                physics::sph_xsph_system,
                physics::mhd_induction_system,
                physics::divergence_cleaning_system,
                physics::compute_lorentz_factor_system,
                physics::relativistic_momentum_system,
                physics::compute_doppler_shift_system,
                physics::sector_boundary_system,
                physics::update_momentum_system,
                physics::compute_angular_acceleration_system,
                physics::damping_system,
                physics::rotational_integration_system,
                physics::lense_thirring_system,
                physics::narrow_phase_system,
                physics::collision_impulse_system,
            ).chain().after(physics::velocity_verlet_velocity_system));

            // Pass 2c — energy + thermodynamics (7 systems)
            // collision_impulse_system appears only once in this branch → unambiguous
            schedule.add_systems((
                physics::compute_kinetic_energy_system,
                physics::compute_potential_energy_system,
                physics::energy_drift_monitor_system,
                physics::sync_temperature_system,
                physics::heat_conduction_system,
                physics::radiative_cooling_system,
                physics::sync_internal_energy_system,
            ).chain().after(physics::collision_impulse_system));
        }
        IntegrationMethod::RungeKutta4 => {
            // Group 1: force evaluation (16 systems)
            schedule.add_systems((
                physics::reset_forces,
                physics::reset_torques,
                physics::build_broadphase_system,
                physics::broadphase_query_system,
                physics::brute_force_gravity_system,
                physics::nfw_gravity_system,
                physics::coulomb_force_system,
                physics::lorentz_force_system,
                physics::geodesic_correction_system,
                physics::sph_density_system,
                physics::sph_eos_system,
                physics::sph_pressure_force_system,
                physics::sph_viscosity_system,
                physics::sph_boundary_system,
                physics::compute_acceleration_system,
            ).chain());

            // Group 2: integration + post-integration (8 systems, after group 1)
            schedule.add_systems((
                physics::rk4_system,
                physics::sph_xsph_system,
                physics::mhd_induction_system,
                physics::divergence_cleaning_system,
                physics::compute_lorentz_factor_system,
                physics::relativistic_momentum_system,
                physics::compute_doppler_shift_system,
                physics::sector_boundary_system,
            ).chain().after(physics::compute_acceleration_system));

            // Group 3: rest of pipeline (14 systems)
            schedule.add_systems((
                physics::update_momentum_system,
                physics::compute_angular_acceleration_system,
                physics::damping_system,
                physics::rotational_integration_system,
                physics::lense_thirring_system,
                physics::narrow_phase_system,
                physics::collision_impulse_system,
                physics::compute_kinetic_energy_system,
                physics::compute_potential_energy_system,
                physics::energy_drift_monitor_system,
                physics::sync_temperature_system,
                physics::heat_conduction_system,
                physics::radiative_cooling_system,
                physics::sync_internal_energy_system,
            ).chain().after(physics::sector_boundary_system));
        }
    }

    // --- Main Loop ---
    // For now, run a single tick to verify setup.
    // The full game loop with winit will be added in Sandbox Phase S1.
    schedule.run(&mut world);

    println!("phys-rs engine initialized. Schedule executed successfully.");
}
