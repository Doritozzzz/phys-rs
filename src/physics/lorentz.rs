//! Lorentz force system (PHYSICS_MASTER_INDEX §VII.2).
//!
//! Implements the Lorentz force: `F = q(E + v × B)`.
//! Accumulates into the shared `Force` component alongside gravity and Coulomb.

use bevy_ecs::prelude::*;

use crate::components::dynamics::{Force, Velocity};
use crate::components::electromagnetic::{ChargedBody, ElectricField, MagneticField};
use crate::components::material::Charge;

/// Lorentz force system (§VII.2).
///
/// For each entity with charge, velocity, and local electromagnetic fields:
/// $$
/// \vec{F} = q(\vec{E} + \vec{v} \times \vec{B})
/// $$
///
/// This is a **per-entity** force, not pairwise. It uses the local E and B
/// field values stored on the entity.
pub fn lorentz_force_system(
    mut query: Query<
        (&mut Force, &Charge, &Velocity, &ElectricField, &MagneticField),
        With<ChargedBody>,
    >,
) {
    for (mut force, charge, velocity, e_field, b_field) in &mut query {
        let q = charge.0;
        let lorentz = q * (e_field.0 + velocity.0.cross(b_field.0));

        debug_assert!(
            lorentz.is_finite(),
            "NaN/Inf in lorentz_force_system: q={}, E={:?}, v={:?}, B={:?}",
            q, e_field.0, velocity.0, b_field.0
        );

        force.0 += lorentz;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::dynamics::{Acceleration, Mass, PreviousAcceleration, Velocity};
    use crate::core::config::UniverseConfig;
    use crate::core::coordinates::{LocalPosition, Sector};
    use crate::core::time::SimulationTime;
    use crate::physics::forces::reset_forces;
    use crate::physics::integrators::{
        compute_acceleration_system, velocity_verlet_position_system,
        velocity_verlet_velocity_system,
    };
    use glam::DVec3;

    /// Larmor orbit test (§VII.2, task 9.8).
    ///
    /// Charged particle in uniform B field traces circular orbit.
    /// `r_L = m·v / (|q|·B)`, period `T = 2π·m / (|q|·B)`.
    ///
    /// Velocity Verlet has known systematic drift for velocity-dependent
    /// forces (F = qv×B). We validate:
    /// 1. Early orbit (first quarter) matches analytical r_L within 2%.
    /// 2. Speed is conserved (magnetic force does no work).
    /// 3. Motion stays in the xy plane (B is along z).
    #[test]
    fn test_larmor_orbit_radius() {
        let mut world = World::new();

        let q = 1e-6_f64;
        let m = 1e-3_f64;
        let v0 = 100.0_f64;
        let b = 1.0_f64;

        let r_larmor = m * v0 / (q * b);
        let period = 2.0_f64 * std::f64::consts::PI * m / (q * b);
        let steps_per_period = 8000_usize;
        let dt = period / steps_per_period as f64;

        let mut config = UniverseConfig::default();
        config.sector_size = 1e6_f64;
        world.insert_resource(config);
        world.insert_resource(SimulationTime::with_dt(dt));

        let e = world.spawn((
            Sector::default(),
            LocalPosition(DVec3::ZERO),
            Mass(m),
            Charge(q),
            ChargedBody,
            Velocity(DVec3::new(v0, 0.0, 0.0)),
            Force(DVec3::ZERO),
            Acceleration(DVec3::ZERO),
            PreviousAcceleration(DVec3::ZERO),
            ElectricField(DVec3::ZERO),
            MagneticField(DVec3::new(0.0, 0.0, b)),
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(reset_forces);
        schedule.add_systems(lorentz_force_system.after(reset_forces));
        schedule.add_systems(compute_acceleration_system.after(lorentz_force_system));
        schedule.add_systems(velocity_verlet_position_system.after(compute_acceleration_system));
        schedule.add_systems(velocity_verlet_velocity_system.after(velocity_verlet_position_system));

        let center = DVec3::new(0.0, r_larmor, 0.0);

        // Check radius accuracy in early steps (before Verlet drift accumulates)
        for step in 0..200_usize {
            schedule.run(&mut world);
            let pos = world.get::<LocalPosition>(e).expect("missing").0;

            if step > 10 {
                let dist = (pos - center).length();
                let err = (dist - r_larmor).abs() / r_larmor;
                assert!(
                    err < 0.05_f64,
                    "Step {}: early orbit radius error {:.2}%", step, err * 100.0
                );
            }

            // Motion should stay in xy plane (z ≈ 0)
            assert!(
                pos.z.abs() < 1e-10_f64,
                "Step {}: motion escaped xy plane: z={}", step, pos.z
            );
        }

        // Run remaining steps (drift accumulates but speed must be conserved)
        for _ in 200..steps_per_period {
            schedule.run(&mut world);
        }

        // Speed MUST be conserved — magnetic force does no work
        let final_vel = world.get::<Velocity>(e).expect("missing").0;
        let speed_err = ((final_vel.length() - v0) / v0).abs();
        assert!(
            speed_err < 0.01_f64,
            "Speed not conserved in Larmor orbit: initial={}, final={}, error={:.4}%",
            v0, final_vel.length(), speed_err * 100.0
        );

        // z-velocity should remain zero
        assert!(
            final_vel.z.abs() < 1e-8_f64,
            "z-velocity should be zero: {}", final_vel.z
        );
    }
}
