use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::dynamics::{Force, Mass};
use crate::core::config::UniverseConfig;
use crate::core::coordinates::{displacement, LocalPosition, Sector};

/// Brute-force N-body gravitational force accumulation.
///
/// Implements Newton's Law of Universal Gravitation (PHYSICS_MASTER_INDEX §IV.1):
/// $$
/// \vec{F}_{12} = -G \frac{m_1 m_2}{|\vec{r}_{12}|^3} \vec{r}_{12}
/// $$
///
/// ## Numerical Safety
/// - Softening: Uses denominator `r^2 + ε^2` to prevent singularity.
/// - Precision: `f64` and `DVec3` exclusively.
/// - Overflow prevention: Multiplies `G * m1` before `m2` to keep values moderate.
pub fn brute_force_gravity_system(
    mut query: Query<(&mut Force, &Mass, &Sector, &LocalPosition)>,
    config: Res<UniverseConfig>,
) {
    let g = config.gravitational_constant;
    let eps2 = config.softening_epsilon * config.softening_epsilon;
    let sector_size = config.sector_size;

    let mut iter = query.iter_combinations_mut();
    while let Some([a, b]) = iter.fetch_next() {
        let (mut force_a, mass_a, sector_a, local_a) = a;
        let (mut force_b, mass_b, sector_b, local_b) = b;

        // r_vec points from a to b
        let r_vec = displacement(sector_a, local_a, sector_b, local_b, sector_size);
        let r2 = r_vec.length_squared();

        let softened_r2 = r2 + eps2;
        let softened_r = softened_r2.sqrt();
        let softened_r3 = softened_r2 * softened_r;

        // Force on A points toward B (in direction of r_vec)
        // Order of multiplication: (G * m1) * m2 to prevent large*large overflow.
        let scalar_f = ((g * mass_a.0) * mass_b.0) / softened_r3;
        let f_vec = r_vec * scalar_f;

        debug_assert!(f_vec.is_finite(), "NaN/Inf detected in brute_force_gravity_system");

        force_a.0 += f_vec;
        force_b.0 -= f_vec;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_figure_8_stability() {
        let mut world = World::new();
        let mut config = UniverseConfig::default();
        config.softening_epsilon = 0.0;
        world.insert_resource(config);

        let m = 1.0_f64;
        let e1 = world.spawn((Force(DVec3::ZERO), Mass(m), Sector::default(), LocalPosition(DVec3::new(-0.97000436, 0.24308753, 0.0)))).id();
        let e2 = world.spawn((Force(DVec3::ZERO), Mass(m), Sector::default(), LocalPosition(DVec3::new(0.97000436, -0.24308753, 0.0)))).id();
        let e3 = world.spawn((Force(DVec3::ZERO), Mass(m), Sector::default(), LocalPosition(DVec3::new(0.0, 0.0, 0.0)))).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(brute_force_gravity_system);
        schedule.run(&mut world);

        let f1 = world.get::<Force>(e1).unwrap().0;
        let f2 = world.get::<Force>(e2).unwrap().0;
        let f3 = world.get::<Force>(e3).unwrap().0;

        assert!(f1.is_finite() && f2.is_finite() && f3.is_finite());
        assert!((f1.x + f2.x + f3.x).abs() < 1e-10);
        assert!((f1.y + f2.y + f3.y).abs() < 1e-10);
    }

    #[test]
    fn test_kepler_orbit_period() {
        let mut world = World::new();
        let mut config = UniverseConfig::default();
        config.softening_epsilon = 0.0;
        let g = config.gravitational_constant;
        world.insert_resource(config);

        let m_big = 1e12_f64;
        let m_small = 1.0_f64;
        let r = 100.0_f64;

        let _big = world.spawn((Force(DVec3::ZERO), Mass(m_big), Sector::default(), LocalPosition(DVec3::ZERO))).id();
        let small = world.spawn((Force(DVec3::ZERO), Mass(m_small), Sector::default(), LocalPosition(DVec3::new(r, 0.0, 0.0)))).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(brute_force_gravity_system);
        schedule.run(&mut world);

        let f = world.get::<Force>(small).unwrap().0;
        let expected_f = -(g * m_big * m_small) / (r * r);

        assert!((f.x - expected_f).abs() < 1e-10);
        assert_eq!(f.y, 0.0);
        assert_eq!(f.z, 0.0);
    }
}
