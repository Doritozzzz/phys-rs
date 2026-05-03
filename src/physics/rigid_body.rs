use bevy_ecs::prelude::*;

use crate::components::dynamics::Velocity;
use crate::components::rotational::{
    AngularDamping, AngularVelocity, InertiaTensor, LinearDamping, Torque,
};
use crate::core::time::SimulationTime;

/// Computes angular acceleration and updates angular velocity.
///
/// Implements `α = I⁻¹(τ - ω × (Iω))`
/// (PHYSICS_MASTER_INDEX §II.2)
pub fn compute_angular_acceleration_system(
    time: Res<SimulationTime>,
    mut query: Query<(&mut AngularVelocity, &InertiaTensor, &Torque)>,
) {
    let dt = time.dt;
    for (mut ang_vel, inertia, torque) in &mut query {
        let i_mat = glam::DMat3::from_cols_array(&inertia.0);
        let w = ang_vel.0;
        
        // Iω
        let i_w = i_mat * w;
        
        // Gyroscopic term: ω × (Iω)
        let gyro = w.cross(i_w);
        
        // Net torque: τ - ω × (Iω)
        let net_torque = torque.0 - gyro;
        
        // Angular acceleration: α = I⁻¹(net_torque)
        let i_inv = i_mat.inverse();
        let alpha = i_inv * net_torque;
        
        // Update angular velocity: ω_{new} = ω + α * dt
        ang_vel.0 += alpha * dt;
        
        debug_assert!(ang_vel.0.is_finite(), "NaN/Inf in angular velocity");
    }
}

/// Applies exponential damping to linear and angular velocities.
pub fn damping_system(
    time: Res<SimulationTime>,
    mut query_lin: Query<(&mut Velocity, &LinearDamping)>,
    mut query_ang: Query<(&mut AngularVelocity, &AngularDamping)>,
) {
    let dt = time.dt;
    
    // Linear damping
    for (mut vel, damp) in &mut query_lin {
        if damp.0 > 0.0 {
            vel.0 *= 1.0 - (damp.0 * dt).min(1.0);
        }
    }
    
    // Angular damping
    for (mut ang_vel, damp) in &mut query_ang {
        if damp.0 > 0.0 {
            ang_vel.0 *= 1.0 - (damp.0 * dt).min(1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    
    #[test]
    fn test_angular_momentum_conservation() {
        let mut world = World::new();
        world.insert_resource(SimulationTime::with_dt(0.001));
        
        // Solid box: larger I_y and I_z than I_x
        let inertia = InertiaTensor::solid_box(10.0, 1.0, 2.0, 3.0);
        let i_mat = glam::DMat3::from_cols_array(&inertia.0);
        
        // Initial angular velocity
        let w0 = DVec3::new(1.0, 2.0, 3.0);
        let l0 = i_mat * w0; // Initial angular momentum L = Iω
        
        let entity = world.spawn((
            AngularVelocity(w0),
            inertia,
            Torque(DVec3::ZERO), // Torque-free
        )).id();
        
        let mut schedule = Schedule::default();
        schedule.add_systems(compute_angular_acceleration_system);
        
        // Run for a few steps
        for _ in 0..1000 {
            schedule.run(&mut world);
        }
        
        let w_final = world.get::<AngularVelocity>(entity).unwrap().0;
        let l_final = i_mat * w_final;
        
        // Angular momentum magnitude should be conserved in torque-free rotation
        // The direction will change (gyroscopic precession / Dzhanibekov effect)
        let drift = (l_final.length() - l0.length()).abs();
        
        // Explicit Euler accumulates error over time, so we tolerate a small numerical drift
        assert!(drift < 0.1, "Angular momentum magnitude not conserved: drift = {}", drift);
        
        // Kinetic energy should also be approximately conserved
        let ke0 = 0.5 * w0.dot(l0);
        let ke_final = 0.5 * w_final.dot(l_final);
        let ke_drift = (ke_final - ke0).abs();
        assert!(ke_drift < 0.1, "Rotational kinetic energy not conserved: drift = {}", ke_drift);
    }
}
