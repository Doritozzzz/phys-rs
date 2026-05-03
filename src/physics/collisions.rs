use bevy_ecs::prelude::*;
use glam::DVec3;
use parry3d_f64::math::{Pose, Rotation};
use parry3d_f64::query::contact;

use crate::components::collisions::{
    CoefficientOfRestitution, CollisionEvent, CollisionEvents, CollisionShape,
};
use crate::components::dynamics::{Mass, Velocity};
use crate::components::rotational::{AngularVelocity, InertiaTensor, Orientation};
use crate::core::config::UniverseConfig;
use crate::core::coordinates::{LocalPosition, Sector, displacement};
use crate::physics::broadphase::CandidatePairs;

/// Narrow-phase collision system.
///
/// Takes broadphase candidate pairs, computes exact geometric intersections
/// using parry3d, and emits `CollisionEvent`s.
pub fn narrow_phase_system(
    query: Query<(&Sector, &LocalPosition, &Orientation, &CollisionShape)>,
    candidate_pairs: Res<CandidatePairs>,
    mut collision_events: ResMut<CollisionEvents>,
    config: Res<UniverseConfig>,
) {
    let sector_size = config.sector_size;

    for pair in candidate_pairs.0.iter() {
        if let (Ok(a), Ok(b)) = (query.get(pair.entity_a), query.get(pair.entity_b)) {
            let (sector_a, local_a, rot_a, shape_a) = a;
            let (sector_b, local_b, rot_b, shape_b) = b;

            let pos_a = DVec3::ZERO; // Treat A as origin for this pair calculation
            let pos_b = displacement(sector_a, local_a, sector_b, local_b, sector_size);

            let iso_a = Pose::from_parts(
                parry3d_f64::math::Vector::new(pos_a.x, pos_a.y, pos_a.z),
                Rotation::from_xyzw(rot_a.0.x, rot_a.0.y, rot_a.0.z, rot_a.0.w),
            );

            let iso_b = Pose::from_parts(
                parry3d_f64::math::Vector::new(pos_b.x, pos_b.y, pos_b.z),
                Rotation::from_xyzw(rot_b.0.x, rot_b.0.y, rot_b.0.z, rot_b.0.w),
            );

            // Prediction of 0.0 means we only report touching or intersecting
            if let Ok(Some(contact)) = contact(&iso_a, &*shape_a.0, &iso_b, &*shape_b.0, 0.0) {
                if contact.dist <= 0.0 {
                    let normal =
                        DVec3::new(contact.normal1.x, contact.normal1.y, contact.normal1.z);
                    let point = DVec3::new(contact.point1.x, contact.point1.y, contact.point1.z);

                    collision_events.0.push(CollisionEvent {
                        entity_a: pair.entity_a,
                        entity_b: pair.entity_b,
                        normal,
                        point,
                        impulse_magnitude: 0.0, // Calculated in the impulse solver
                    });
                }
            }
        }
    }
}

/// Computes and applies collision impulses for intersecting rigid bodies.
///
/// PHYSICS_MASTER_INDEX §III.1
pub fn collision_impulse_system(
    mut events: ResMut<CollisionEvents>,
    mut query: Query<(
        &mut Velocity,
        &mut AngularVelocity,
        &Mass,
        Option<&InertiaTensor>,
        Option<&CoefficientOfRestitution>,
        Option<&crate::components::collisions::FrictionCoefficient>,
        &Sector,
        &LocalPosition,
    )>,
    config: Res<UniverseConfig>,
) {
    let sector_size = config.sector_size;

    for event in events.0.iter_mut() {
        let Ok([mut a, mut b]) = query.get_many_mut([event.entity_a, event.entity_b]) else {
            continue;
        };

        let (mut vel_a, mut ang_vel_a, mass_a, inertia_a, rest_a, fric_a, sec_a, loc_a) = a;
        let (mut vel_b, mut ang_vel_b, mass_b, inertia_b, rest_b, fric_b, sec_b, loc_b) = b;

        // Minimum restitution coefficient
        let e = rest_a
            .map(|r| r.0)
            .unwrap_or(0.5)
            .min(rest_b.map(|r| r.0).unwrap_or(0.5));
            
        // Combined friction coefficient (typically multiplied, but min is also common)
        let mu_k = fric_a
            .map(|f| f.0)
            .unwrap_or(0.3)
            .min(fric_b.map(|f| f.0).unwrap_or(0.3));

        let pos_a = DVec3::ZERO;
        let pos_b = displacement(sec_a, loc_a, sec_b, loc_b, sector_size);

        let r_a = event.point - pos_a;
        let r_b = event.point - pos_b;

        let v_ap = vel_a.0 + ang_vel_a.0.cross(r_a);
        let v_bp = vel_b.0 + ang_vel_b.0.cross(r_b);

        let v_rel = v_ap - v_bp;
        let rel_vel_normal = v_rel.dot(event.normal);

        // Do not resolve if objects are already separating
        // v_rel.dot(normal) > 0 means approaching, < 0 means separating.
        if rel_vel_normal < 0.0 {
            continue;
        }

        let inv_mass_a = 1.0 / mass_a.0;
        let inv_mass_b = 1.0 / mass_b.0;

        let inv_inertia_a = inertia_a
            .map(|i| glam::DMat3::from_cols_array(&i.0).inverse())
            .unwrap_or(glam::DMat3::ZERO);

        let inv_inertia_b = inertia_b
            .map(|i| glam::DMat3::from_cols_array(&i.0).inverse())
            .unwrap_or(glam::DMat3::ZERO);

        // --- NORMAL IMPULSE ---
        
        let r_a_cross_n = r_a.cross(event.normal);
        let r_b_cross_n = r_b.cross(event.normal);

        let vec_i_inv_a_n = inv_inertia_a * r_a_cross_n;
        let vec_i_inv_b_n = inv_inertia_b * r_b_cross_n;

        let rot_term_a_n = vec_i_inv_a_n.cross(r_a).dot(event.normal);
        let rot_term_b_n = vec_i_inv_b_n.cross(r_b).dot(event.normal);

        let j_n = -(1.0 + e) * rel_vel_normal / (inv_mass_a + inv_mass_b + rot_term_a_n + rot_term_b_n);
        let impulse_n = event.normal * j_n;
        
        // --- TANGENTIAL (FRICTION) IMPULSE ---
        
        let v_tangential = v_rel - event.normal * rel_vel_normal;
        
        let mut impulse_t = DVec3::ZERO;
        let mut vec_i_inv_a_t = DVec3::ZERO;
        let mut vec_i_inv_b_t = DVec3::ZERO;
        let mut j_t = 0.0;
        
        // Only compute friction if tangential velocity is significant
        if v_tangential.length_squared() > 1e-8 {
            let tangent = v_tangential.normalize();
            let rel_vel_tangent = v_rel.dot(tangent);
            
            let r_a_cross_t = r_a.cross(tangent);
            let r_b_cross_t = r_b.cross(tangent);

            vec_i_inv_a_t = inv_inertia_a * r_a_cross_t;
            vec_i_inv_b_t = inv_inertia_b * r_b_cross_t;

            let rot_term_a_t = vec_i_inv_a_t.cross(r_a).dot(tangent);
            let rot_term_b_t = vec_i_inv_b_t.cross(r_b).dot(tangent);
            
            let j_t_unclamped = -rel_vel_tangent / (inv_mass_a + inv_mass_b + rot_term_a_t + rot_term_b_t);
            
            // Coulomb friction clamping
            let max_friction = mu_k * j_n.abs();
            j_t = j_t_unclamped.clamp(-max_friction, max_friction);
            impulse_t = tangent * j_t;
        }

        // --- APPLY IMPULSES ---
        
        let total_impulse = impulse_n + impulse_t;

        vel_a.0 += total_impulse * inv_mass_a;
        vel_b.0 -= total_impulse * inv_mass_b;

        ang_vel_a.0 += vec_i_inv_a_n * j_n + vec_i_inv_a_t * j_t;
        ang_vel_b.0 -= vec_i_inv_b_n * j_n + vec_i_inv_b_t * j_t;
    }

    // Clear events after processing
    events.0.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use crate::components::dynamics::{Mass, Velocity};
    use crate::components::rotational::{AngularVelocity, InertiaTensor};
    use crate::components::collisions::CoefficientOfRestitution;

    fn setup_world() -> World {
        let mut world = World::new();
        world.insert_resource(UniverseConfig::default());
        world.insert_resource(CollisionEvents::default());
        world
    }

    #[test]
    fn test_head_on_sphere_collision() {
        let mut world = setup_world();

        // Sphere A (moving right)
        let entity_a = world.spawn((
            Velocity(DVec3::new(10.0, 0.0, 0.0)),
            AngularVelocity(DVec3::ZERO),
            Mass(5.0),
            Sector::ORIGIN,
            LocalPosition::ZERO,
            CoefficientOfRestitution(1.0), // perfectly elastic
        )).id();

        // Sphere B (moving left)
        let entity_b = world.spawn((
            Velocity(DVec3::new(-10.0, 0.0, 0.0)),
            AngularVelocity(DVec3::ZERO),
            Mass(5.0),
            Sector::ORIGIN,
            LocalPosition::ZERO,
            CoefficientOfRestitution(1.0), // perfectly elastic
        )).id();

        // Inject a collision event
        world.resource_mut::<CollisionEvents>().0.push(CollisionEvent {
            entity_a,
            entity_b,
            normal: DVec3::new(1.0, 0.0, 0.0), // normal from A to B
            point: DVec3::ZERO,
            impulse_magnitude: 0.0,
        });

        // Run the system manually (we create a small schedule)
        let mut schedule = Schedule::default();
        schedule.add_systems(collision_impulse_system);
        schedule.run(&mut world);

        // Verify velocities exchanged
        let v_a = world.get::<Velocity>(entity_a).unwrap().0;
        let v_b = world.get::<Velocity>(entity_b).unwrap().0;

        assert!((v_a.x - (-10.0)).abs() < 1e-5, "A should bounce back with -10 m/s, got {}", v_a.x);
        assert!((v_b.x - 10.0).abs() < 1e-5, "B should bounce back with 10 m/s, got {}", v_b.x);
    }

    #[test]
    fn test_restitution_behavior() {
        let mut world = setup_world();

        // Ball A (falling)
        let entity_a = world.spawn((
            Velocity(DVec3::new(0.0, -10.0, 0.0)),
            AngularVelocity(DVec3::ZERO),
            Mass(1.0),
            Sector::ORIGIN,
            LocalPosition::ZERO,
            CoefficientOfRestitution(0.5), // half elastic
        )).id();

        // Ground B (infinite mass -> very large mass)
        let entity_b = world.spawn((
            Velocity(DVec3::ZERO),
            AngularVelocity(DVec3::ZERO),
            Mass(1e12),
            Sector::ORIGIN,
            LocalPosition::ZERO,
            CoefficientOfRestitution(0.5), // half elastic
        )).id();

        // Inject collision event (normal points UP)
        world.resource_mut::<CollisionEvents>().0.push(CollisionEvent {
            entity_a,
            entity_b,
            normal: DVec3::new(0.0, -1.0, 0.0), // Normal from A (falling) to B (ground) is DOWN
            point: DVec3::ZERO,
            impulse_magnitude: 0.0,
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(collision_impulse_system);
        schedule.run(&mut world);

        // Verify A bounces up with 0.5 * 10 = 5 m/s
        let v_a = world.get::<Velocity>(entity_a).unwrap().0;
        assert!((v_a.y - 5.0).abs() < 1e-3, "A should bounce up with 5 m/s, got {}", v_a.y);
    }
}
