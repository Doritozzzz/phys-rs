//! General relativistic effects (PHYSICS_MASTER_INDEX §X.2).
//!
//! Implements:
//! - Schwarzschild geodesic corrections (task 10.6)
//! - Photon ray-marching in curved spacetime (task 10.6)
//! - Lense-Thirring frame-dragging precession (task 10.7)

use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::dynamics::{Force, Mass, Velocity};
use crate::components::rotational::AngularVelocity;
use crate::core::constants::{C, G};
use crate::core::coordinates::LocalPosition;

/// Speed of light squared.
const C2: f64 = C * C;

// ─── Marker Component ──────────────────────────────────────────────

/// Marker for entities that receive GR corrections (post-Newtonian terms).
///
/// Only entities with this component will have their acceleration
/// modified by the geodesic correction system. Use for bodies
/// orbiting near compact objects (neutron stars, black holes).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RelativisticBody;

/// Angular momentum vector J of a massive rotating body [kg m² s⁻¹].
///
/// Used by the Lense-Thirring system to compute frame-dragging.
/// Attach to the central massive body (e.g., a rotating black hole).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AngularMomentumJ(pub DVec3);

// ─── Utility Functions ─────────────────────────────────────────────

/// Schwarzschild effective potential for orbital mechanics (§X.2).
///
/// $$
/// V_{eff}(r) = -\frac{GM}{r} + \frac{L^2}{2r^2} - \frac{GML^2}{c^2 r^3}
/// $$
///
/// The last term is the GR correction. In Newtonian gravity, only
/// the first two terms exist. The GR term creates an unstable
/// circular orbit for photons at r = 1.5 r_s (photon sphere).
/// Schwarzschild effective potential for massive particles (§X.2).
pub fn schwarzschild_effective_potential(
    r: f64,
    specific_angular_momentum: f64,
    mass_central: f64,
) -> f64 {
    let l2 = specific_angular_momentum * specific_angular_momentum;
    let gm = G * mass_central;

    let newtonian = -gm / r + l2 / (2.0_f64 * r * r);
    let gr_correction = -gm * l2 / (C2 * r * r * r);

    newtonian + gr_correction
}

/// Schwarzschild effective potential for photons (null geodesics).
pub fn schwarzschild_effective_potential_photon(
    r: f64,
    specific_angular_momentum: f64,
    mass_central: f64,
) -> f64 {
    let l2 = specific_angular_momentum * specific_angular_momentum;
    let gm = G * mass_central;

    // Photons do not have the -GM/r mass term
    let centrifugal = l2 / (2.0_f64 * r * r);
    let gr_correction = -gm * l2 / (C2 * r * r * r);

    centrifugal + gr_correction
}

/// Leading-order post-Newtonian GR correction to acceleration (§X.2).
///
/// For a body orbiting mass M at position r with velocity v:
/// $$
/// \vec{a}_{GR} = \frac{GM}{c^2 r^3} \left[
///     \left(4\frac{GM}{r} - v^2\right)\vec{r} + 4(\vec{r}\cdot\vec{v})\vec{v}
/// \right]
/// $$
///
/// This is the 1PN (first post-Newtonian) correction from the
/// Einstein-Infeld-Hoffmann equations. It produces perihelion
/// precession and captures the photon sphere.
pub fn post_newtonian_acceleration(
    pos: DVec3,
    vel: DVec3,
    mass_central: f64,
) -> DVec3 {
    let r = pos.length();
    if r < 1e-10_f64 {
        return DVec3::ZERO;
    }

    let gm = G * mass_central;
    let r2 = r * r;
    let r3 = r2 * r;
    let v2 = vel.length_squared();
    let r_dot_v = pos.dot(vel);

    let factor = gm / (C2 * r3);
    let a_gr = factor * ((4.0_f64 * gm / r - v2) * pos + 4.0_f64 * r_dot_v * vel);

    debug_assert!(a_gr.is_finite(), "Post-Newtonian acceleration NaN/Inf");
    a_gr
}

/// Lense-Thirring frame-dragging precession rate (§X.2, task 10.7).
///
/// $$
/// \vec{\Omega}_{LT} = \frac{G}{c^2 r^3}
///     \left[ \frac{3(\vec{J}\cdot\hat{r})\hat{r}}{1} - \vec{J} \right]
/// $$
///
/// Returns the precession angular velocity vector that the orbiting
/// body's angular velocity should be corrected by.
pub fn lense_thirring_precession(
    pos: DVec3,
    angular_momentum_j: DVec3,
) -> DVec3 {
    let r = pos.length();
    if r < 1e-10_f64 {
        return DVec3::ZERO;
    }

    let r_hat = pos / r;
    let r3 = r * r * r;

    let factor = G / (C2 * r3);
    let j_dot_rhat = angular_momentum_j.dot(r_hat);

    let omega_lt = factor * (3.0_f64 * j_dot_rhat * r_hat - angular_momentum_j);

    debug_assert!(omega_lt.is_finite(), "Lense-Thirring NaN/Inf");
    omega_lt
}

/// Photon geodesic step in Schwarzschild geometry (task 10.6).
///
/// Advances a photon's position and velocity by one step `ds` using
/// the post-Newtonian acceleration. Photons travel at c and experience
/// gravitational deflection.
///
/// Returns (new_pos, new_vel) with |new_vel| renormalized to c.
pub fn photon_geodesic_step(
    pos: DVec3,
    vel: DVec3,
    mass_central: f64,
    ds: f64,
) -> (DVec3, DVec3) {
    let r = pos.length();
    if r < 1e-10_f64 {
        return (pos, vel);
    }

    // Using the exact effective potential force for null geodesics:
    // a = -3GM / (c^2 r^5) * |r x v|^2 * r
    // This perfectly reproduces the photon sphere at r = 1.5 r_s.
    let gm = G * mass_central;
    let r5 = r.powi(5);
    let l2 = pos.cross(vel).length_squared();
    
    let a_photon = -3.0_f64 * gm / (C2 * r5) * l2 * pos;

    // Semi-implicit Euler step (velocity first, for stability)
    let new_vel = vel + a_photon * ds;
    // Renormalize to speed of light (photons always travel at c)
    let new_vel = new_vel.normalize() * C;
    let new_pos = pos + new_vel * ds;

    debug_assert!(new_pos.is_finite(), "Photon position NaN");
    debug_assert!(new_vel.is_finite(), "Photon velocity NaN");

    (new_pos, new_vel)
}

// ─── ECS Systems ───────────────────────────────────────────────────

/// GR geodesic correction system (§X.2, task 10.6).
///
/// Adds post-Newtonian acceleration corrections to relativistic bodies
/// orbiting massive objects. This runs alongside Newtonian gravity —
/// it does NOT replace it.
///
/// Currently uses a simplified single-central-body model where the
/// most massive entity acts as the source of spacetime curvature.
pub fn geodesic_correction_system(
    mut bodies: Query<(&mut Force, &Mass, &LocalPosition, &Velocity), With<RelativisticBody>>,
    massive: Query<(&Mass, &LocalPosition), Without<RelativisticBody>>,
) {
    // Find the most massive non-relativistic body as the curvature source
    let mut max_mass = 0.0_f64;
    let mut central_pos = DVec3::ZERO;
    let mut central_mass = 0.0_f64;

    for (mass, pos) in &massive {
        if mass.0 > max_mass {
            max_mass = mass.0;
            central_mass = mass.0;
            central_pos = pos.0;
        }
    }

    if central_mass < 1e-10_f64 {
        return;
    }

    for (mut force, mass, pos, vel) in &mut bodies {
        let rel_pos = pos.0 - central_pos;
        let a_gr = post_newtonian_acceleration(rel_pos, vel.0, central_mass);
        force.0 += a_gr * mass.0;

        debug_assert!(
            force.0.is_finite(),
            "Force NaN after GR correction"
        );
    }
}

/// Lense-Thirring frame-dragging system (§X.2, task 10.7).
///
/// For each entity near a massive rotating body, computes the
/// frame-dragging precession and applies it as an angular velocity
/// correction.
pub fn lense_thirring_system(
    mut bodies: Query<(&mut AngularVelocity, &LocalPosition), With<RelativisticBody>>,
    sources: Query<(&AngularMomentumJ, &LocalPosition)>,
    time: Res<crate::core::time::SimulationTime>,
) {
    let dt = time.dt;

    for (mut ang_vel, body_pos) in &mut bodies {
        for (j, source_pos) in &sources {
            let rel_pos = body_pos.0 - source_pos.0;
            let omega_lt = lense_thirring_precession(rel_pos, j.0);

            // Apply as angular velocity correction over dt
            ang_vel.0 += omega_lt * dt;

            debug_assert!(
                ang_vel.0.is_finite(),
                "Angular velocity NaN after Lense-Thirring"
            );
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::constants::SOLAR_MASS;
    use crate::physics::orbital::schwarzschild_radius;

    /// Photon circular orbit at r = 1.5·r_s (photon sphere, task 10.9).
    ///
    /// A photon launched tangentially at the photon sphere radius
    /// should maintain approximately constant distance from the
    /// central mass over a partial orbit.
    #[test]
    fn test_photon_sphere_orbit() {
        let m = 10.0_f64 * SOLAR_MASS; // 10 solar mass BH
        let r_s = schwarzschild_radius(m);
        let r_photon = 1.5_f64 * r_s; // photon sphere

        // Photon starts at (r_photon, 0, 0) moving tangentially in y
        let mut pos = DVec3::new(r_photon, 0.0, 0.0);
        let mut vel = DVec3::new(0.0, C, 0.0); // speed of light, tangential

        // Step size: small fraction of the light-crossing time of the orbit
        let circumference = 2.0_f64 * std::f64::consts::PI * r_photon;
        let orbital_time = circumference / C;
        let ds = orbital_time / 10000.0_f64;

        let mut max_r = 0.0_f64;
        let mut min_r = f64::MAX;

        // Run for half an orbit
        let steps = 5000_usize;
        for _ in 0..steps {
            (pos, vel) = photon_geodesic_step(pos, vel, m, ds);
            let r = pos.length();
            max_r = max_r.max(r);
            min_r = min_r.min(r);
        }

        // The photon should stay near r_photon (within 20%)
        // This is a marginally stable orbit, so some drift is expected
        let drift_max = (max_r - r_photon).abs() / r_photon;
        let drift_min = (min_r - r_photon).abs() / r_photon;

        assert!(
            drift_max < 0.20_f64,
            "Photon drifted too far outward: max_r={:.6e}, r_photon={:.6e}, drift={:.2}%",
            max_r, r_photon, drift_max * 100.0
        );
        assert!(
            drift_min < 0.20_f64,
            "Photon drifted too far inward: min_r={:.6e}, r_photon={:.6e}, drift={:.2}%",
            min_r, r_photon, drift_min * 100.0
        );
    }

    /// GR correction direction.
    ///
    /// For a highly relativistic particle (v ~ c), the EIH correction points INWARD
    /// adding to gravity. For slow particles, it points OUTWARD.
    #[test]
    fn test_gr_correction_direction() {
        let m_central = SOLAR_MASS;
        let r = 1e6_f64; // very close, highly relativistic

        let pos = DVec3::new(r, 0.0, 0.0);
        let vel = DVec3::new(0.0, 0.99_f64 * C, 0.0); // almost speed of light

        let a_gr = post_newtonian_acceleration(pos, vel, m_central);

        // With v = 0.99c, the v^2 term dominates (4GM/r - v^2 is negative)
        // So a_gr points inward (negative x)
        assert!(
            a_gr.x < 0.0_f64,
            "Highly relativistic GR correction should point inward: a_gr={:?}", a_gr
        );
    }

    /// Lense-Thirring precession should be perpendicular to both
    /// J and r for equatorial orbits.
    #[test]
    fn test_lense_thirring_equatorial() {
        // J along z-axis, body in the equatorial plane
        let j = DVec3::new(0.0, 0.0, 1e40_f64); // massive spin
        let pos = DVec3::new(1e6_f64, 0.0, 0.0);

        let omega = lense_thirring_precession(pos, j);

        // For equatorial orbit: Ω_LT should be along +z
        // (3(J·r̂)r̂ - J) = 3*0*r̂ - J_z ẑ = -J_z ẑ)
        // Wait: J·r̂ = 0 for equatorial, so Ω = (G/c²r³)(-J) → along -z
        assert!(
            omega.z < 0.0_f64,
            "Equatorial LT should precess opposite to J: Ω={:?}", omega
        );

        // x and y components should be zero (within floating point)
        assert!(
            omega.x.abs() < 1e-30_f64 && omega.y.abs() < 1e-30_f64,
            "Equatorial LT should only have z-component: Ω={:?}", omega
        );
    }

    /// Effective potential should have a local maximum at the photon sphere.
    #[test]
    fn test_effective_potential_photon_sphere() {
        let m = SOLAR_MASS;
        let r_s = schwarzschild_radius(m);

        // For a photon, use L such that the circular orbit is at 1.5 r_s
        // V_eff'(r) = 0 at r = 1.5 r_s for the appropriate L
        // L² = 3 G M r_s / c² = 3 G M (2GM/c²) / c² = 6 G²M²/c⁴
        // Actually: for photons, L = √(27) GM/c (specific ang. mom.)
        let l = (27.0_f64 as f64).sqrt() * G * m / C;

        let r_photon = 1.5_f64 * r_s;

        // V_eff should be at a local maximum
        let dr = r_photon * 0.001_f64;
        let v_left = schwarzschild_effective_potential_photon(r_photon - dr, l, m);
        let v_center = schwarzschild_effective_potential_photon(r_photon, l, m);
        let v_right = schwarzschild_effective_potential_photon(r_photon + dr, l, m);

        assert!(
            v_center > v_left && v_center > v_right,
            "V_eff should have max at photon sphere: V_left={:.6e}, V_center={:.6e}, V_right={:.6e}",
            v_left, v_center, v_right
        );
    }
}
