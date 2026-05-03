use bevy_ecs::prelude::*;

use crate::components::dynamics::{Mass, Velocity};
use crate::core::config::UniverseConfig;
use crate::core::coordinates::{distance_squared, LocalPosition, Sector};

/// Energy monitor resource. Tracks total system energy to detect conservation violations.
#[derive(Resource, Debug, Default)]
pub struct EnergyMonitor {
    /// Total kinetic energy (E_k) [J]
    pub kinetic_energy: f64,
    /// Total gravitational potential energy (U) [J]
    pub potential_energy: f64,
    /// Total mechanical energy (E_k + U) [J]
    pub total_energy: f64,
    /// Total energy at the start of the simulation [J]
    pub initial_energy: Option<f64>,
}

/// Computes the total kinetic energy of all bodies: `E_k = 1/2 m v^2`.
pub fn compute_kinetic_energy_system(
    query: Query<(&Mass, &Velocity)>,
    mut monitor: ResMut<EnergyMonitor>,
) {
    let mut total_ke = 0.0;
    for (mass, velocity) in query.iter() {
        total_ke += 0.5 * mass.0 * velocity.0.length_squared();
    }
    monitor.kinetic_energy = total_ke;
}

/// Computes the total gravitational potential energy: `U = -G * m1 * m2 / r`.
/// Uses `iter_combinations` to evaluate all pairs.
pub fn compute_potential_energy_system(
    query: Query<(&Mass, &Sector, &LocalPosition)>,
    config: Res<UniverseConfig>,
    mut monitor: ResMut<EnergyMonitor>,
) {
    let g = config.gravitational_constant;
    let sector_size = config.sector_size;
    
    let mut total_pe = 0.0;
    let mut iter = query.iter_combinations();
    while let Some([a, b]) = iter.fetch_next() {
        let (mass_a, sector_a, local_a) = a;
        let (mass_b, sector_b, local_b) = b;
        
        let r2 = distance_squared(sector_a, local_a, sector_b, local_b, sector_size);
        if r2 > 0.0 {
            let r = r2.sqrt();
            // U = -G * m1 * m2 / r
            total_pe -= (g * mass_a.0 * mass_b.0) / r;
        }
    }
    monitor.potential_energy = total_pe;
}

/// Monitors total energy drift and issues a warning if conservation is violated.
pub fn energy_drift_monitor_system(
    mut monitor: ResMut<EnergyMonitor>,
) {
    monitor.total_energy = monitor.kinetic_energy + monitor.potential_energy;
    
    if monitor.initial_energy.is_none() && monitor.total_energy != 0.0 {
        monitor.initial_energy = Some(monitor.total_energy);
    }
    
    if let Some(initial) = monitor.initial_energy {
        if initial.abs() > 0.0 {
            let drift = (monitor.total_energy - initial).abs() / initial.abs();
            if drift > 1e-4 {
                // Warning threshold crossed
                eprintln!("WARNING: Energy conservation violated! Drift: {:.5e} (Threshold: 1e-4)", drift);
            }
        }
    }
}
