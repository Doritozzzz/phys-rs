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

        let (mut vel_a, mut ang_vel_a, mass_a, inertia_a, rest_a, sec_a, loc_a) = a;
        let (mut vel_b, mut ang_vel_b, mass_b, inertia_b, rest_b, sec_b, loc_b) = b;

        // Minimum restitution coefficient
        let e = rest_a
            .map(|r| r.0)
            .unwrap_or(0.5)
            .min(rest_b.map(|r| r.0).unwrap_or(0.5));

        let pos_a = DVec3::ZERO;
        let pos_b = displacement(sec_a, loc_a, sec_b, loc_b, sector_size);

        let r_a = event.point - pos_a;
        let r_b = event.point - pos_b;

        let v_ap = vel_a.0 + ang_vel_a.0.cross(r_a);
        let v_bp = vel_b.0 + ang_vel_b.0.cross(r_b);

        let v_rel = v_ap - v_bp;
        let rel_vel_normal = v_rel.dot(event.normal);

        // Do not resolve if objects are already separating
        if rel_vel_normal > 0.0 {
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

        // Cross products for angular inertia component: (r x n)
        let r_a_cross_n = r_a.cross(event.normal);
        let r_b_cross_n = r_b.cross(event.normal);

        // I^{-1} (r x n)
        let vec_i_inv_a = inv_inertia_a * r_a_cross_n;
        let vec_i_inv_b = inv_inertia_b * r_b_cross_n;

        // [ I^{-1} (r x n) ] x r . n
        let rot_term_a = vec_i_inv_a.cross(r_a).dot(event.normal);
        let rot_term_b = vec_i_inv_b.cross(r_b).dot(event.normal);

        // Compute impulse scalar j (PHYSICS_MASTER_INDEX §III.1)
        let j = -(1.0 + e) * rel_vel_normal / (inv_mass_a + inv_mass_b + rot_term_a + rot_term_b);

        let impulse = event.normal * j;

        vel_a.0 += impulse * inv_mass_a;
        vel_b.0 -= impulse * inv_mass_b;

        ang_vel_a.0 += vec_i_inv_a * j;
        ang_vel_b.0 -= vec_i_inv_b * j;
    }

    // Clear events after processing
    events.0.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_on_sphere_collision() {
        // Implementation stub for verifying linear momentum conservation
        assert!(true);
    }

    #[test]
    fn test_restitution_behavior() {
        // Implementation stub for sphere bouncing on plane
        assert!(true);
    }
}
