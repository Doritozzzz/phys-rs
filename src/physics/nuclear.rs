//! Nuclear physics and stellar evolution.
//!
//! Implements Section IX of `PHYSICS_MASTER_INDEX.md`.

use bevy_ecs::prelude::*;
use crate::core::constants::{C, CHANDRASEKHAR_LIMIT, K_B, PROTON_MASS, HBAR, SOLAR_MASS, ELEMENTARY_CHARGE, COULOMB_CONSTANT};
use crate::components::dynamics::Mass;
use crate::components::nuclear::{StellarCore, StellarState, SupernovaEvent, SupernovaEvents};
use crate::components::identifiers::{BodyType, EntityName};
use std::f64::consts::PI;

/// Calculates energy released from mass defect.
///
/// Implements Mass-Energy Equivalence (PHYSICS_MASTER_INDEX §IX.1):
/// `E = Δm·c²`
pub fn mass_energy_equivalence(delta_m: f64) -> f64 {
    delta_m * C * C
}

/// Calculates approximate Gamow peak reaction rate.
///
/// Implements Gamow Peak approximation (PHYSICS_MASTER_INDEX §IX.1).
/// This is a highly simplified version for the engine.
pub fn gamow_reaction_rate(temperature: f64, density: f64, z_a: f64, z_b: f64, m_r: f64) -> f64 {
    if temperature <= 0.0 {
        return 0.0;
    }
    
    let k_e_e2 = COULOMB_CONSTANT * ELEMENTARY_CHARGE * ELEMENTARY_CHARGE;
    
    // The exponent depends on T^(-1/3)
    let exponent = -3.0 * ( (PI * PI * z_a * z_a * z_b * z_b * k_e_e2 * k_e_e2 * m_r) / (2.0 * HBAR * HBAR * K_B * temperature) ).powf(1.0/3.0);
    
    // Rate is proportional to density squared and T^(-2/3)
    let rate = density * density * temperature.powf(-2.0/3.0) * exponent.exp();
    
    // Ensure finite
    if rate.is_finite() { rate } else { 0.0 }
}

/// Calculates electron degeneracy pressure for a Fermi gas.
///
/// Implements Degeneracy Pressure limit (PHYSICS_MASTER_INDEX §IX.2):
/// `P ∝ ρ^(5/3)`
pub fn degeneracy_pressure(density: f64) -> f64 {
    // Proportionality constant for non-relativistic electron degeneracy pressure.
    // P = K * rho^(5/3)
    let k = 1.0036e7_f64; // Approximate constant for electron fraction Ye = 0.5
    k * density.powf(5.0 / 3.0)
}

/// Evaluates stellar lifecycle states and triggers supernovae if Chandrasekhar limit is exceeded.
pub fn stellar_lifecycle_system(
    mut query: Query<(Entity, &Mass, &mut StellarState, &mut BodyType, Option<&EntityName>)>,
    mut supernova_events: ResMut<SupernovaEvents>,
) {
    for (entity, mass, mut state, mut body_type, _name) in query.iter_mut() {
        // Only care about remnants and late-stage stars for collapse
        if *state == StellarState::WhiteDwarf || *state == StellarState::RedGiant {
            // Check Chandrasekhar limit
            if mass.0 > CHANDRASEKHAR_LIMIT {
                // Determine remnant type based on mass
                // For simplicity: < 3 M_sun -> Neutron Star, else Black Hole
                let remnant = if mass.0 < 3.0 * SOLAR_MASS {
                    StellarState::NeutronStar
                } else {
                    StellarState::BlackHole
                };
                
                // Update ECS components
                *state = remnant;
                
                *body_type = match remnant {
                    StellarState::NeutronStar => BodyType::NeutronStar,
                    StellarState::BlackHole => BodyType::BlackHole,
                    _ => *body_type, // Should not happen
                };
                
                // Fire event
                supernova_events.0.push(SupernovaEvent {
                    entity,
                    core_mass: mass.0,
                    remnant_type: remnant,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_energy_equivalence() {
        let e = mass_energy_equivalence(1.0); // 1 kg
        let expected = C * C;
        assert!((e - expected).abs() < 1e-5);
    }

    #[test]
    fn test_gamow_reaction_rate() {
        let t1 = 1.5e7;
        let t2 = 1.6e7;
        let rho = 1.5e5;
        let z_a = 1.0;
        let z_b = 1.0;
        let m_r = PROTON_MASS / 2.0;

        let rate1 = gamow_reaction_rate(t1, rho, z_a, z_b, m_r);
        let rate2 = gamow_reaction_rate(t2, rho, z_a, z_b, m_r);
        
        // Rate should increase strongly with T
        assert!(rate2 > rate1);
    }

    #[test]
    fn test_degeneracy_pressure() {
        let p1 = degeneracy_pressure(1.0e6);
        let p2 = degeneracy_pressure(2.0e6);
        
        let ratio = p2 / p1;
        let expected_ratio = 2.0_f64.powf(5.0/3.0);
        assert!((ratio - expected_ratio).abs() < 1e-5);
    }

    #[test]
    fn test_chandrasekhar_collapse() {
        let mut world = World::new();
        world.insert_resource(SupernovaEvents::default());
        
        let entity = world.spawn((
            Mass(3.0e30), // > Chandrasekhar limit
            StellarState::WhiteDwarf,
            BodyType::WhiteDwarf,
        )).id();
        
        let mut schedule = Schedule::default();
        schedule.add_systems(stellar_lifecycle_system);
        schedule.run(&mut world);
        
        let events = world.resource::<SupernovaEvents>();
        assert_eq!(events.0.len(), 1);
        assert_eq!(events.0[0].remnant_type, StellarState::NeutronStar);
        
        let state = world.get::<StellarState>(entity).unwrap();
        assert_eq!(*state, StellarState::NeutronStar);
    }
}
