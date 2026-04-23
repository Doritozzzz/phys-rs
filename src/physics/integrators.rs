//! Numerical integration systems.
//!
//! Implements three integration methods from PHYSICS_MASTER_INDEX §I:
//! - Semi-Implicit Euler (§I.1) — symplectic, first-order
//! - Velocity Verlet (§I.2) — symplectic, second-order, energy-conserving
//! - RK4 (§I.3) — fourth-order accuracy
//!
//! All integrators use `f64` precision and include `debug_assert!`
//! finiteness checks on every integrated value.

use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::{Acceleration, Force, Mass, PreviousAcceleration, Velocity};
use crate::core::coordinates::LocalPosition;
use crate::core::SimulationTime;

/// Semi-Implicit Euler integrator (PHYSICS_MASTER_INDEX §I.1).
///
/// ```text
/// v_{t+dt} = v_t + a_t * dt       (velocity FIRST)
/// r_{t+dt} = r_t + v_{t+dt} * dt  (position uses NEW velocity)
/// ```
///
/// ## Numerical Safety
/// - Precision: f64 / DVec3 exclusively
/// - Finiteness: `debug_assert!` on velocity and position after update
///
/// CRITICAL: The order matters — updating position with the OLD velocity
/// would be standard (non-symplectic) Euler, which is energy-divergent.
pub fn semi_implicit_euler_system(
    time: Res<SimulationTime>,
    mut query: Query<(&mut Velocity, &mut LocalPosition, &Acceleration)>,
) {
    let dt = time.dt;
    for (mut vel, mut pos, acc) in &mut query {
        // Step 1: Update velocity with current acceleration
        vel.0 += acc.0 * dt;
        debug_assert!(vel.0.is_finite(), "NaN/Inf in velocity after Semi-Implicit Euler");

        // Step 2: Update position with NEW velocity (symplectic order)
        pos.0 += vel.0 * dt;
        debug_assert!(pos.0.is_finite(), "NaN/Inf in position after Semi-Implicit Euler");
    }
}

/// Velocity Verlet integrator (PHYSICS_MASTER_INDEX §I.2).
///
/// ```text
/// r_{t+dt} = r_t + v_t * dt + 0.5 * a_t * dt²
/// a_{t+dt} = F(r_{t+dt}) / m     (recomputed by force systems)
/// v_{t+dt} = v_t + 0.5 * (a_t + a_{t+dt}) * dt
/// ```
///
/// This system handles the POSITION update and stores the current
/// acceleration as `PreviousAcceleration`. The VELOCITY half-step
/// is completed by [`velocity_verlet_velocity_system`] after forces
/// are recalculated at the new position.
///
/// ## Numerical Safety
/// - Precision: f64 / DVec3 exclusively
/// - Finiteness: `debug_assert!` on position after update
pub fn velocity_verlet_position_system(
    time: Res<SimulationTime>,
    mut query: Query<(
        &Velocity,
        &mut LocalPosition,
        &Acceleration,
        &mut PreviousAcceleration,
    )>,
) {
    let dt = time.dt;
    let half_dt_sq = 0.5_f64 * dt * dt;

    for (vel, mut pos, acc, mut prev_acc) in &mut query {
        // Store current acceleration for the velocity half-step
        prev_acc.0 = acc.0;

        // Update position: r_new = r + v*dt + 0.5*a*dt²
        pos.0 += vel.0 * dt + acc.0 * half_dt_sq;
        debug_assert!(pos.0.is_finite(), "NaN/Inf in position after Verlet position step");
    }
}

/// Velocity Verlet velocity half-step (PHYSICS_MASTER_INDEX §I.2).
///
/// Completes the velocity update using the average of old and new accelerations:
/// `v_{t+dt} = v_t + 0.5 * (a_old + a_new) * dt`
///
/// Must run AFTER force recalculation at the new position and AFTER
/// acceleration is recomputed from the new forces.
pub fn velocity_verlet_velocity_system(
    time: Res<SimulationTime>,
    mut query: Query<(&mut Velocity, &Acceleration, &PreviousAcceleration)>,
) {
    let dt = time.dt;
    let half_dt = 0.5_f64 * dt;

    for (mut vel, acc, prev_acc) in &mut query {
        // v_new = v + 0.5 * (a_old + a_new) * dt
        vel.0 += (prev_acc.0 + acc.0) * half_dt;
        debug_assert!(vel.0.is_finite(), "NaN/Inf in velocity after Verlet velocity step");
    }
}

/// Compute acceleration from accumulated force: `a = F / m` (Newton's Second Law, §II.1).
///
/// Runs after all force accumulator systems and before integration
/// (or between Verlet position and velocity steps).
pub fn compute_acceleration_system(
    mut query: Query<(&mut Acceleration, &Force, &Mass)>,
) {
    for (mut acc, force, mass) in &mut query {
        debug_assert!(mass.0 > 0.0_f64, "Zero or negative mass detected");
        acc.0 = force.0 / mass.0;
        debug_assert!(acc.0.is_finite(), "NaN/Inf in acceleration (F/m)");
    }
}

/// Update linear momentum from mass and velocity: `p = m * v`.
///
/// Runs after integration for conservation tracking.
pub fn update_momentum_system(
    mut query: Query<(&mut crate::components::LinearMomentum, &Mass, &Velocity)>,
) {
    for (mut momentum, mass, vel) in &mut query {
        momentum.0 = vel.0 * mass.0;
    }
}

// =============================================================================
// RK4 — More complex, requires temporary state
// =============================================================================

/// Runge-Kutta 4th Order integrator (PHYSICS_MASTER_INDEX §I.3).
///
/// ```text
/// k1 = f(t, y)
/// k2 = f(t + dt/2, y + dt/2 * k1)
/// k3 = f(t + dt/2, y + dt/2 * k2)
/// k4 = f(t + dt, y + dt * k3)
/// y_{n+1} = y_n + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
/// ```
///
/// For a particle with state `(position, velocity)` under constant
/// acceleration within the timestep, this evaluates the derivative
/// at four points and combines them with the standard RK4 weights.
///
/// NOTE: True RK4 requires re-evaluating forces at intermediate states.
/// This simplified version uses the current acceleration as constant
/// across the step, which is valid for small `dt` and smooth force fields.
/// For full RK4 with force re-evaluation, use the staged pipeline.
pub fn rk4_system(
    time: Res<SimulationTime>,
    mut query: Query<(&mut Velocity, &mut LocalPosition, &Acceleration)>,
) {
    let dt = time.dt;

    for (mut vel, mut pos, acc) in &mut query {
        let v0 = vel.0;
        let r0 = pos.0;
        let a = acc.0; // Acceleration treated as constant across the step

        // State: (r, v). Derivative: (v, a).
        // k1
        let k1_v = a;
        let k1_r = v0;

        // k2
        let k2_v = a; // a is constant
        let k2_r = v0 + k1_v * (dt * 0.5_f64);

        // k3
        let k3_v = a;
        let k3_r = v0 + k2_v * (dt * 0.5_f64);

        // k4
        let k4_v = a;
        let k4_r = v0 + k3_v * dt;

        // Combine
        let sixth_dt = dt / 6.0_f64;
        vel.0 = v0 + (k1_v + k2_v * 2.0_f64 + k3_v * 2.0_f64 + k4_v) * sixth_dt;
        pos.0 = r0 + (k1_r + k2_r * 2.0_f64 + k3_r * 2.0_f64 + k4_r) * sixth_dt;

        debug_assert!(vel.0.is_finite(), "NaN/Inf in velocity after RK4");
        debug_assert!(pos.0.is_finite(), "NaN/Inf in position after RK4");
    }
}

/// Rotational integration: updates orientation quaternion from angular velocity.
///
/// Implements PHYSICS_MASTER_INDEX §II.3:
/// ```text
/// q_{t+dt} = q_t + 0.5 * [0, ω] ⊗ q_t * dt
/// ```
///
/// Re-normalizes the quaternion after each step to prevent drift.
pub fn rotational_integration_system(
    time: Res<SimulationTime>,
    mut query: Query<(
        &mut crate::components::Orientation,
        &crate::components::AngularVelocity,
    )>,
) {
    let dt = time.dt;

    for (mut orientation, angular_vel) in &mut query {
        let q = orientation.0;
        let w = angular_vel.0;

        // Quaternion derivative: dq/dt = 0.5 * ω_quat * q
        // where ω_quat = DQuat(w.x, w.y, w.z, 0)
        let omega_quat = glam::DQuat::from_xyzw(w.x, w.y, w.z, 0.0_f64);
        let dq = (omega_quat * q) * 0.5_f64;

        // Euler step on quaternion
        let new_q = glam::DQuat::from_xyzw(
            q.x + dq.x * dt,
            q.y + dq.y * dt,
            q.z + dq.z * dt,
            q.w + dq.w * dt,
        );

        // Re-normalize to maintain unit quaternion
        orientation.0 = new_q.normalize();

        debug_assert!(orientation.0.is_finite(), "NaN/Inf in orientation after rotational integration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::*;
    use crate::core::coordinates::*;

    /// Helper: create a minimal world with one body and run a system.
    fn setup_single_body(
        dt: f64,
        pos: DVec3,
        vel: DVec3,
        acc: DVec3,
    ) -> World {
        let mut world = World::new();
        world.insert_resource(SimulationTime::with_dt(dt));
        world.spawn((
            LocalPosition(pos),
            Velocity(vel),
            Acceleration(acc),
            Force(DVec3::ZERO),
            Mass(1.0_f64),
            PreviousAcceleration(DVec3::ZERO),
            LinearMomentum(DVec3::ZERO),
        ));
        world
    }

    #[test]
    fn test_semi_euler_uniform_acceleration() {
        // Object under constant acceleration: a = (0, -9.81, 0)
        let dt = 0.01_f64;
        let mut world = setup_single_body(
            dt,
            DVec3::ZERO,
            DVec3::new(10.0, 0.0, 0.0),
            DVec3::new(0.0, -9.81, 0.0),
        );

        let mut schedule = Schedule::default();
        schedule.add_systems(semi_implicit_euler_system);

        // Run 100 steps (1 second)
        for _ in 0..100 {
            schedule.run(&mut world);
            // Advance tick
            world.resource_mut::<SimulationTime>().advance_tick();
        }

        let mut query = world.query::<(&Velocity, &LocalPosition)>();
        for (vel, pos) in query.iter(&world) {
            // After 1s: vy ≈ -9.81, x ≈ 10.0
            assert!((vel.0.y - (-9.81_f64)).abs() < 0.1);
            assert!((pos.0.x - 10.0_f64).abs() < 0.2);
        }
    }

    #[test]
    fn test_verlet_energy_conservation_circular_orbit() {
        // Two-body: Earth-Moon circular orbit simplified
        // Use unit system where G*M = 1, r = 1 → v = 1, T = 2π
        let dt = 0.001_f64;
        let mut world = World::new();
        world.insert_resource(SimulationTime::with_dt(dt));

        // Single body orbiting a fixed center at origin
        // Circular orbit: r=1, v=1 (perpendicular to r)
        world.spawn((
            LocalPosition(DVec3::new(1.0, 0.0, 0.0)),
            Velocity(DVec3::new(0.0, 1.0, 0.0)),
            Acceleration(DVec3::new(-1.0, 0.0, 0.0)), // centripetal: -r/|r|³ with GM=1
            Force(DVec3::ZERO),
            Mass(1.0_f64),
            PreviousAcceleration(DVec3::new(-1.0, 0.0, 0.0)),
            LinearMomentum(DVec3::ZERO),
        ));

        // Compute initial energy: E = 0.5*v² - GM/r = 0.5 - 1.0 = -0.5
        let initial_energy = -0.5_f64;

        let mut schedule = Schedule::default();
        schedule.add_systems(velocity_verlet_position_system);

        // We'll manually recompute centripetal acceleration and then do velocity step
        let steps = 6283; // ~1 full orbit (2π / 0.001)
        for _ in 0..steps {
            schedule.run(&mut world);

            // Recompute acceleration: a = -r/|r|³ (with GM=1)
            let mut query = world.query::<(&LocalPosition, &mut Acceleration)>();
            for (pos, mut acc) in query.iter_mut(&mut world) {
                let r = pos.0;
                let r_mag = r.length();
                acc.0 = -r / (r_mag * r_mag * r_mag);
            }

            // Velocity half-step
            let mut vel_schedule = Schedule::default();
            vel_schedule.add_systems(velocity_verlet_velocity_system);
            vel_schedule.run(&mut world);

            world.resource_mut::<SimulationTime>().advance_tick();
        }

        // Check energy conservation
        let mut query = world.query::<(&Velocity, &LocalPosition)>();
        for (vel, pos) in query.iter(&world) {
            let ke = 0.5_f64 * vel.0.length_squared();
            let pe = -1.0_f64 / pos.0.length();
            let final_energy = ke + pe;
            let drift = (final_energy - initial_energy).abs();
            // Verlet should conserve energy to < 1e-4 over one orbit
            assert!(
                drift < 1e-4,
                "Energy drift too large: {drift} (initial={initial_energy}, final={final_energy})"
            );
        }
    }
}
