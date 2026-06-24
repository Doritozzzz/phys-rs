//! # phys-rs — Universal Physics Engine
//!
//! Entry point. Sets up the ECS [`World`], inserts resources,
//! configures the physics [`Schedule`], and launches the winit event loop.

mod components;
mod core;
mod gpu;
mod physics;
mod render;
mod scenarios;

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
    world.insert_resource(crate::gpu::GpuConfig::default());
    world.insert_resource(crate::core::diagnostics::Diagnostics::default());

    // --- GPU Init ---
    let mut init_schedule = Schedule::default();
    init_schedule.add_systems(crate::gpu::try_init_gpu);
    init_schedule.run(&mut world);

    // --- Spawn test entities ---
    scenarios::spawn_test_demo(&mut world);

    // --- Schedule Setup ---
    let mut schedule = Schedule::default();

    match integration_method {
        IntegrationMethod::SemiImplicitEuler => {
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

    // --- Sandbox Event Loop ---
    let mut app = render::App::new(world, schedule);
    let event_loop = winit::event_loop::EventLoop::new()
        .expect("Failed to create winit event loop");
    event_loop
        .run_app(&mut app)
        .expect("Event loop terminated with error");
}
