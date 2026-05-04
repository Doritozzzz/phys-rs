//! Electrostatic force systems (PHYSICS_MASTER_INDEX §VII.2).
//!
//! Implements:
//! - Coulomb's law with softening for pairwise charged-particle interactions.
//! - Charge-based force accumulation into the shared `Force` component,
//!   running alongside gravity in the force pipeline.

use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::dynamics::Force;
use crate::components::electromagnetic::ChargedBody;
use crate::components::material::Charge;
use crate::core::config::UniverseConfig;
use crate::core::constants::COULOMB_CONSTANT;
use crate::core::coordinates::{displacement, LocalPosition, Sector};

/// Brute-force pairwise Coulomb force accumulation (§VII.2).
///
/// Implements Coulomb's law with softening:
/// $$
/// \vec{F}_{12} = k_e \frac{q_1 q_2}{(r^2 + \varepsilon_{em}^2)^{3/2}} \vec{r}_{12}
/// $$
///
/// ## Sign Convention
/// - `r_vec = pos_b - pos_a` — vector from A to B.
/// - Like charges (`q₁·q₂ > 0`) → force is repulsive (A pushed away from B).
/// - Unlike charges (`q₁·q₂ < 0`) → force is attractive (A pulled toward B).
///
/// ## Numerical Safety
/// - Softening: denominator `r² + ε_em²` prevents singularity at r = 0.
/// - Overflow prevention: multiplies `(k_e * q1) * q2` to keep intermediates moderate.
/// - `debug_assert!` on finiteness after each pair computation.
///
/// ## Newton's Third Law
/// Each pair is computed once; `+F` applied to A, `−F` applied to B.
pub fn coulomb_force_system(
    mut query: Query<(&mut Force, &Charge, &Sector, &LocalPosition), With<ChargedBody>>,
    config: Res<UniverseConfig>,
) {
    let eps2 = config.em_softening_epsilon * config.em_softening_epsilon;
    let sector_size = config.sector_size;

    let mut iter = query.iter_combinations_mut();
    while let Some([a, b]) = iter.fetch_next() {
        let (mut force_a, charge_a, sector_a, local_a) = a;
        let (mut force_b, charge_b, sector_b, local_b) = b;

        // Skip if either body is electrically neutral
        if charge_a.0 == 0.0_f64 || charge_b.0 == 0.0_f64 {
            continue;
        }

        // r_vec points from A to B
        let r_vec = displacement(sector_a, local_a, sector_b, local_b, sector_size);
        let r2 = r_vec.length_squared();

        let softened_r2 = r2 + eps2;
        let softened_r = softened_r2.sqrt();
        let softened_r3 = softened_r2 * softened_r;

        // Coulomb scalar: k_e * q1 * q2 / (r² + ε²)^(3/2)
        // Multiplication order: (k_e * q1) * q2 to prevent overflow.
        let scalar_f = ((COULOMB_CONSTANT * charge_a.0) * charge_b.0) / softened_r3;

        // Force vector: positive scalar with like charges → pushes A away from B
        // (r_vec points toward B, so force on A is +r_vec * scalar for repulsion).
        // Coulomb naturally handles this: like charges → scalar > 0 → repulsive.
        let f_vec = r_vec * scalar_f;

        debug_assert!(
            f_vec.is_finite(),
            "NaN/Inf detected in coulomb_force_system: q1={}, q2={}, r²={}, f={:?}",
            charge_a.0, charge_b.0, r2, f_vec
        );

        // Newton's Third Law: equal and opposite
        // NOTE: Unlike gravity (always attractive), Coulomb sign depends on
        // charge product. F_on_A = -k_e * q_A * q_B / r³ * r_vec (the minus
        // ensures like charges repel and unlike attract).
        force_a.0 -= f_vec;
        force_b.0 += f_vec;
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::dynamics::{Acceleration, Force, Mass, PreviousAcceleration, Velocity};
    use crate::core::coordinates::Sector;
    use crate::core::time::SimulationTime;
    use crate::physics::integrators::{compute_acceleration_system, velocity_verlet_position_system, velocity_verlet_velocity_system};
    use crate::physics::forces::reset_forces;
    use glam::DVec3;

    /// Coulomb orbit test (§VII.2, task 9.7).
    ///
    /// Two oppositely charged particles placed in a circular orbit.
    /// Circular orbit condition: `k_e |q₁ q₂| / r² = m v² / r`
    /// → `v = √(k_e |q₁ q₂| / (m r))`
    ///
    /// After one full orbital period, verifies:
    /// - Distance doesn't drift more than 10% from initial separation.
    /// - Total energy (kinetic + Coulomb potential) stays within threshold.
    #[test]
    fn test_coulomb_orbit_stability() {
        let mut world = World::new();

        let q = 1e-6_f64;          // 1 μC
        let m = 1e-3_f64;          // 1 g
        let r0 = 0.1_f64;          // 10 cm separation

        // Circular orbit velocity: v = sqrt(k_e * |q1 * q2| / (m * r))
        // For the reduced mass problem, each particle orbits the center of mass.
        // With equal masses, each is at r0/2 from center, orbital radius = r0/2.
        // Centripetal: k_e * q² / r0² = m * v² / (r0/2)
        // → v = sqrt(k_e * q² / (2 * m * r0))
        let v_orbit = (COULOMB_CONSTANT * q * q / (2.0_f64 * m * r0)).sqrt();

        // Orbital period: T = 2π (r0/2) / v = π r0 / v
        let orbital_period = std::f64::consts::PI * r0 / v_orbit;

        // Use small dt for accuracy (5000 steps per orbit)
        let dt = orbital_period / 5000.0_f64;

        let mut config = UniverseConfig::default();
        config.sector_size = 1e6_f64;
        config.em_softening_epsilon = 1e-10_f64; // very small softening for accuracy
        world.insert_resource(config);
        world.insert_resource(SimulationTime::with_dt(dt));

        // Particle A at (-r0/2, 0, 0), charge +q, velocity (0, -v, 0)
        let ea = world.spawn((
            Sector::default(),
            LocalPosition(DVec3::new(-r0 / 2.0_f64, 0.0, 0.0)),
            Mass(m),
            Charge(q),
            ChargedBody,
            Velocity(DVec3::new(0.0, -v_orbit, 0.0)),
            Force(DVec3::ZERO),
            Acceleration(DVec3::ZERO),
            PreviousAcceleration(DVec3::ZERO),
        )).id();

        // Particle B at (+r0/2, 0, 0), charge -q, velocity (0, +v, 0)
        let eb = world.spawn((
            Sector::default(),
            LocalPosition(DVec3::new(r0 / 2.0_f64, 0.0, 0.0)),
            Mass(m),
            Charge(-q),
            ChargedBody,
            Velocity(DVec3::new(0.0, v_orbit, 0.0)),
            Force(DVec3::ZERO),
            Acceleration(DVec3::ZERO),
            PreviousAcceleration(DVec3::ZERO),
        )).id();

        // Build schedule: reset → coulomb → accel → verlet position → verlet velocity
        let mut schedule = Schedule::default();
        schedule.add_systems(reset_forces);
        schedule.add_systems(coulomb_force_system.after(reset_forces));
        schedule.add_systems(compute_acceleration_system.after(coulomb_force_system));
        schedule.add_systems(velocity_verlet_position_system.after(compute_acceleration_system));
        schedule.add_systems(velocity_verlet_velocity_system.after(velocity_verlet_position_system));

        // Compute initial energy
        let initial_ke = 0.5_f64 * m * v_orbit * v_orbit * 2.0_f64; // both particles
        let initial_pe = -COULOMB_CONSTANT * q * q / r0; // unlike charges → negative
        let initial_energy = initial_ke + initial_pe;

        let steps = 5000_usize;
        for _ in 0..steps {
            schedule.run(&mut world);
        }

        // Check distance hasn't drifted more than 10%
        let pos_a = world.get::<LocalPosition>(ea).expect("Entity A missing").0;
        let pos_b = world.get::<LocalPosition>(eb).expect("Entity B missing").0;
        let final_r = (pos_b - pos_a).length();

        let drift_pct = ((final_r - r0) / r0).abs();
        assert!(
            drift_pct < 0.10_f64,
            "Coulomb orbit radius drifted {:.2}%: initial={}, final={}",
            drift_pct * 100.0_f64, r0, final_r
        );

        // Check energy conservation
        let vel_a = world.get::<Velocity>(ea).expect("Entity A missing").0;
        let vel_b = world.get::<Velocity>(eb).expect("Entity B missing").0;
        let final_ke = 0.5_f64 * m * vel_a.length_squared()
            + 0.5_f64 * m * vel_b.length_squared();
        let final_pe = -COULOMB_CONSTANT * q * q / final_r;
        let final_energy = final_ke + final_pe;

        let energy_drift = ((final_energy - initial_energy) / initial_energy.abs()).abs();
        assert!(
            energy_drift < 0.01_f64,
            "Coulomb orbit energy drifted {:.4}%: initial={:.6e}, final={:.6e}",
            energy_drift * 100.0_f64, initial_energy, final_energy
        );
    }
}
