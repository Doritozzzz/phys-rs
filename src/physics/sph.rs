//! Smoothed Particle Hydrodynamics (SPH) systems (§V).
//!
//! Implements density estimation, equation of state, pressure gradient
//! forces, and Monaghan artificial viscosity for fluid simulation.

use bevy_ecs::prelude::*;

use crate::components::dynamics::{Force, Mass, Velocity};
use crate::components::sph::{FluidParticle, Pressure, SmoothedDensity, SmoothingRadius};
use crate::core::coordinates::{displacement, LocalPosition, Sector};
use crate::core::config::UniverseConfig;
use crate::physics::broadphase::CandidatePairs;
use std::f64::consts::PI;

// ─── Kernel Functions ──────────────────────────────────────────────

/// Cubic spline SPH kernel W(r, h) in 3D (PHYSICS_MASTER_INDEX §V.1).
///
/// Support domain: `r < 2h`. The kernel is normalized so that
/// `∫ W(r,h) dV = 1` over all 3D space.
///
/// ```text
/// W(q) = σ × { 1 − 3/2 q² + 3/4 q³        if 0 ≤ q < 1
///            { 1/4 (2 − q)³                  if 1 ≤ q < 2
///            { 0                              if q ≥ 2
/// ```
/// where `q = r / h` and `σ = 1 / (π h³)` in 3D.
#[inline]
pub fn cubic_spline_kernel(r: f64, h: f64) -> f64 {
    let q = r / h;
    if q >= 2.0_f64 {
        return 0.0_f64;
    }

    // 3D normalization factor
    let sigma = 1.0_f64 / (PI * h.powi(3));

    if q < 1.0_f64 {
        sigma * (1.0_f64 - 1.5_f64 * q.powi(2) + 0.75_f64 * q.powi(3))
    } else {
        sigma * 0.25_f64 * (2.0_f64 - q).powi(3)
    }
}

/// Radial derivative of the cubic spline kernel dW/dr (§V.1).
///
/// The full vector gradient is `∇W = (dW/dr) × (r̂)` where `r̂ = r⃗ / |r⃗|`.
/// This function returns the scalar `dW/dr`.
#[inline]
pub fn cubic_spline_kernel_gradient(r: f64, h: f64) -> f64 {
    let q = r / h;
    if q >= 2.0_f64 || r < 1e-10_f64 {
        return 0.0_f64;
    }

    // Normalization factor for the gradient (divide by h once more)
    let sigma = 1.0_f64 / (PI * h.powi(4));

    if q < 1.0_f64 {
        sigma * (-3.0_f64 * q + 2.25_f64 * q.powi(2))
    } else {
        sigma * (-0.75_f64 * (2.0_f64 - q).powi(2))
    }
}

// ─── Density System ────────────────────────────────────────────────

/// SPH density estimation system (PHYSICS_MASTER_INDEX §V.1).
///
/// Computes `ρᵢ = Σⱼ mⱼ W(rᵢ − rⱼ, h)` for every fluid particle,
/// **including** self-contribution (`j = i`).
pub fn sph_density_system(
    mut query: Query<(
        Entity,
        &Sector,
        &LocalPosition,
        &Mass,
        &SmoothingRadius,
        &mut SmoothedDensity,
    ), With<FluidParticle>>,
    candidate_pairs: Res<CandidatePairs>,
    config: Res<UniverseConfig>,
) {
    // Self-contribution: W(0, h) for each particle
    for (_, _, _, mass, h, mut density) in &mut query {
        density.0 = mass.0 * cubic_spline_kernel(0.0_f64, h.0);
    }

    // Pairwise contributions from broadphase candidate pairs
    for pair in &candidate_pairs.0 {
        let (e1, e2) = (pair.entity_a, pair.entity_b);

        if let Ok([
            (_, s1, p1, m1, h1, mut d1),
            (_, s2, p2, m2, h2, mut d2),
        ]) = query.get_many_mut([e1, e2])
        {
            let r_vec = displacement(s1, p1, s2, p2, config.sector_size);
            let r = r_vec.length();

            // Average smoothing radius
            let h_avg = 0.5_f64 * (h1.0 + h2.0);

            if r < 2.0_f64 * h_avg {
                let w = cubic_spline_kernel(r, h_avg);

                d1.0 += m2.0 * w;
                d2.0 += m1.0 * w;
            }
        }
    }
}

// ─── Equation of State ─────────────────────────────────────────────

/// Configuration for the Tait equation of state (§V.2).
///
/// Tait EOS for weakly-compressible fluids:
/// `P = B × ((ρ / ρ₀)^γ − 1)` where `B = ρ₀ c² / γ`.
#[derive(Resource, Debug, Clone)]
pub struct TaitEquationConfig {
    /// Reference (rest) density ρ₀ [kg m⁻³].
    pub reference_density: f64,
    /// Speed of sound in the medium c [m s⁻¹].
    pub speed_of_sound: f64,
    /// Stiffness exponent γ (typically 7 for water).
    pub gamma: f64,
}

impl Default for TaitEquationConfig {
    fn default() -> Self {
        Self {
            reference_density: 1000.0_f64, // water density
            speed_of_sound: 1500.0_f64,    // speed of sound in water
            gamma: 7.0_f64,                // standard for weakly-compressible water
        }
    }
}

/// Equation of state system — converts density to pressure (§V.2).
///
/// Uses the Tait equation: `P = B × ((ρ / ρ₀)^γ − 1)`.
/// Negative pressures are clamped to zero to prevent spurious
/// tensile instabilities in the fluid.
pub fn sph_eos_system(
    mut query: Query<(&SmoothedDensity, &mut Pressure), With<FluidParticle>>,
    tait: Res<TaitEquationConfig>,
) {
    let b = tait.reference_density * tait.speed_of_sound.powi(2) / tait.gamma;
    let rho_0 = tait.reference_density;

    for (density, mut pressure) in &mut query {
        let ratio = density.0 / rho_0;
        pressure.0 = b * (ratio.powf(tait.gamma) - 1.0_f64);

        // Clamp negative pressures to prevent tensile instability
        if pressure.0 < 0.0_f64 {
            pressure.0 = 0.0_f64;
        }
    }
}

// ─── Pressure Force System ─────────────────────────────────────────

/// SPH pressure gradient force system (PHYSICS_MASTER_INDEX §V.3).
///
/// Symmetric pressure gradient form that conserves momentum:
/// `F⃗ᵢ = −mᵢ Σⱼ mⱼ (Pᵢ/ρᵢ² + Pⱼ/ρⱼ²) ∇W(r⃗ᵢ − r⃗ⱼ, h)`
pub fn sph_pressure_force_system(
    mut query: Query<(
        Entity,
        &Sector,
        &LocalPosition,
        &Mass,
        &SmoothedDensity,
        &Pressure,
        &SmoothingRadius,
        &mut Force,
    ), With<FluidParticle>>,
    candidate_pairs: Res<CandidatePairs>,
    config: Res<UniverseConfig>,
) {
    for pair in &candidate_pairs.0 {
        let (e1, e2) = (pair.entity_a, pair.entity_b);

        if let Ok([
            (_, s1, p1, m1, d1, pr1, h1, mut f1),
            (_, s2, p2, m2, d2, pr2, h2, mut f2),
        ]) = query.get_many_mut([e1, e2])
        {
            let r_vec = displacement(s1, p1, s2, p2, config.sector_size);
            let r = r_vec.length();

            let h_avg = 0.5_f64 * (h1.0 + h2.0);

            if r > 1e-10_f64 && r < 2.0_f64 * h_avg {
                let r_hat = r_vec / r;
                let dw_dr = cubic_spline_kernel_gradient(r, h_avg);

                // Symmetric pressure term (§V.3)
                let term = (pr1.0 / d1.0.powi(2)) + (pr2.0 / d2.0.powi(2));

                // F = m_i * m_j * term * dW/dr * r̂
                // dW/dr is negative inside the support → force points −r̂ (repulsive). Correct.
                let force_magnitude = m1.0 * m2.0 * term * dw_dr;
                let force_vec = r_hat * force_magnitude;

                f1.0 += force_vec;
                f2.0 -= force_vec;

                debug_assert!(f1.0.is_finite(), "SPH pressure force NaN on entity {:?}", e1);
                debug_assert!(f2.0.is_finite(), "SPH pressure force NaN on entity {:?}", e2);
            }
        }
    }
}

// ─── Viscosity System ──────────────────────────────────────────────

/// Configuration for Monaghan artificial viscosity (§V.3).
#[derive(Resource, Debug, Clone)]
pub struct SphViscosityConfig {
    /// Linear viscosity coefficient α (typical: 0.01–1.0).
    pub alpha: f64,
    /// Quadratic shock-capturing coefficient β (typical: 0–2.0).
    pub beta: f64,
}

impl Default for SphViscosityConfig {
    fn default() -> Self {
        Self {
            alpha: 0.1_f64,
            beta: 0.2_f64,
        }
    }
}

/// Monaghan artificial viscosity system (PHYSICS_MASTER_INDEX §V.3).
///
/// Adds a dissipative term `Πᵢⱼ` to the pressure gradient when
/// particles are **approaching** each other (`v⃗ᵢⱼ · r⃗ᵢⱼ < 0`),
/// preventing particle interpenetration and capturing shocks.
///
/// ```text
/// Πᵢⱼ = (−α c̄ μᵢⱼ + β μᵢⱼ²) / ρ̄ᵢⱼ
/// μᵢⱼ = h v⃗ᵢⱼ·r⃗ᵢⱼ / (|r⃗ᵢⱼ|² + ε h²)
/// ```
pub fn sph_viscosity_system(
    mut query: Query<(
        Entity,
        &Sector,
        &LocalPosition,
        &Velocity,
        &Mass,
        &SmoothedDensity,
        &SmoothingRadius,
        &mut Force,
    ), With<FluidParticle>>,
    candidate_pairs: Res<CandidatePairs>,
    visc: Res<SphViscosityConfig>,
    tait: Res<TaitEquationConfig>,
    config: Res<UniverseConfig>,
) {
    let c = tait.speed_of_sound;
    let epsilon = 1e-2_f64; // prevent division by zero in μ

    for pair in &candidate_pairs.0 {
        let (e1, e2) = (pair.entity_a, pair.entity_b);

        if let Ok([
            (_, s1, p1, v1, m1, d1, h1, mut f1),
            (_, s2, p2, v2, m2, d2, h2, mut f2),
        ]) = query.get_many_mut([e1, e2])
        {
            let r_vec = displacement(s1, p1, s2, p2, config.sector_size);
            let v_rel = v1.0 - v2.0;

            // Only apply viscosity when particles are approaching
            if v_rel.dot(r_vec) < 0.0_f64 {
                let r = r_vec.length();
                let h_avg = 0.5_f64 * (h1.0 + h2.0);

                if r < 2.0_f64 * h_avg {
                    let r_hat = r_vec / r;
                    let dw_dr = cubic_spline_kernel_gradient(r, h_avg);

                    let rho_bar = 0.5_f64 * (d1.0 + d2.0);
                    let mu = (h_avg * v_rel.dot(r_vec))
                        / (r.powi(2) + epsilon * h_avg.powi(2));

                    let pi_ij =
                        (-visc.alpha * c * mu + visc.beta * mu.powi(2)) / rho_bar;

                    let force_magnitude = m1.0 * m2.0 * pi_ij * dw_dr;
                    let force_vec = r_hat * force_magnitude;

                    f1.0 += force_vec;
                    f2.0 -= force_vec;

                    debug_assert!(f1.0.is_finite(), "SPH viscosity NaN on entity {:?}", e1);
                    debug_assert!(f2.0.is_finite(), "SPH viscosity NaN on entity {:?}", e2);
                }
            }
        }
    }
}

// ─── XSPH Velocity Correction ──────────────────────────────────────

/// Configuration for XSPH velocity smoothing.
#[derive(Resource, Debug, Clone)]
pub struct XsphConfig {
    /// XSPH smoothing factor ε ∈ [0, 1]. Higher = more smoothing.
    pub epsilon: f64,
}

impl Default for XsphConfig {
    fn default() -> Self {
        Self {
            epsilon: 0.5_f64,
        }
    }
}

/// XSPH velocity correction system.
///
/// Smooths particle velocities towards the local average, reducing
/// noise and improving visual stability:
/// `v⃗ᵢ_corrected = v⃗ᵢ + ε Σⱼ (mⱼ / ρ̄ᵢⱼ) (v⃗ⱼ − v⃗ᵢ) W(rᵢⱼ, h)`
///
/// Applied **after** force integration to smooth the resulting velocity
/// field without affecting force accumulation.
pub fn sph_xsph_system(
    mut query: Query<(
        Entity,
        &Sector,
        &LocalPosition,
        &Mass,
        &SmoothedDensity,
        &SmoothingRadius,
        &mut Velocity,
    ), With<FluidParticle>>,
    candidate_pairs: Res<CandidatePairs>,
    xsph: Res<XsphConfig>,
    config: Res<UniverseConfig>,
) {
    // Collect velocity corrections first (can't mutate while reading neighbors)
    let mut corrections: Vec<(Entity, glam::DVec3)> = Vec::new();

    for pair in &candidate_pairs.0 {
        let (e1, e2) = (pair.entity_a, pair.entity_b);

        if let Ok([
            (_, s1, p1, m1, d1, h1, v1),
            (_, s2, p2, m2, d2, h2, v2),
        ]) = query.get_many_mut([e1, e2])
        {
            let r_vec = displacement(s1, p1, s2, p2, config.sector_size);
            let r = r_vec.length();
            let h_avg = 0.5_f64 * (h1.0 + h2.0);

            if r < 2.0_f64 * h_avg {
                let w = cubic_spline_kernel(r, h_avg);
                let rho_bar = 0.5_f64 * (d1.0 + d2.0);
                let v_diff = v2.0 - v1.0;

                // Correction for particle 1: towards particle 2's velocity
                let corr1 = xsph.epsilon * (m2.0 / rho_bar) * v_diff * w;
                // Correction for particle 2: towards particle 1's velocity (opposite)
                let corr2 = xsph.epsilon * (m1.0 / rho_bar) * (-v_diff) * w;

                corrections.push((e1, corr1));
                corrections.push((e2, corr2));
            }
        }
    }

    // Apply accumulated corrections
    for (entity, delta_v) in corrections {
        if let Ok((_, _, _, _, _, _, mut vel)) = query.get_mut(entity) {
            vel.0 += delta_v;
            debug_assert!(vel.0.is_finite(), "XSPH produced NaN on entity {:?}", entity);
        }
    }
}

// ─── Boundary Repulsion ────────────────────────────────────────────

/// Configuration for SPH domain boundary enforcement.
///
/// Applies a repulsive force to fluid particles that approach the
/// domain walls, preventing them from escaping without requiring
/// explicit ghost particle spawning.
#[derive(Resource, Debug, Clone)]
pub struct SphBoundaryConfig {
    /// Half-extent of the cubic domain [m]. Domain spans `[-extent, +extent]` on each axis.
    pub half_extent: f64,
    /// Distance from wall at which repulsion activates [m].
    pub activation_distance: f64,
    /// Strength of the repulsive force [N m⁻¹].
    pub stiffness: f64,
}

impl Default for SphBoundaryConfig {
    fn default() -> Self {
        Self {
            half_extent: 10.0_f64,
            activation_distance: 0.5_f64,
            stiffness: 1e6_f64,
        }
    }
}

/// SPH boundary repulsion system.
///
/// For each fluid particle near a domain wall, applies an inward
/// repulsive force proportional to the penetration depth:
/// `F = stiffness × (activation_distance - d)` where `d` is
/// the distance to the nearest wall.
///
/// This is a lightweight alternative to ghost/mirror particles
/// that prevents fluid from escaping the simulation domain.
pub fn sph_boundary_system(
    mut query: Query<(&LocalPosition, &mut Force), With<FluidParticle>>,
    boundary: Res<SphBoundaryConfig>,
) {
    let ext = boundary.half_extent;
    let ad = boundary.activation_distance;
    let k = boundary.stiffness;

    for (pos, mut force) in &mut query {
        let p = pos.0;
        // Check each axis: low wall and high wall
        for axis in 0..3_usize {
            let coord = match axis { 0 => p.x, 1 => p.y, _ => p.z };

            // Distance to low wall (-ext)
            let d_low = coord - (-ext);
            if d_low < ad && d_low >= 0.0_f64 {
                let mag = k * (ad - d_low);
                match axis {
                    0 => force.0.x += mag,
                    1 => force.0.y += mag,
                    _ => force.0.z += mag,
                }
            }

            // Distance to high wall (+ext)
            let d_high = ext - coord;
            if d_high < ad && d_high >= 0.0_f64 {
                let mag = k * (ad - d_high);
                match axis {
                    0 => force.0.x -= mag,
                    1 => force.0.y -= mag,
                    _ => force.0.z -= mag,
                }
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::dynamics::{Acceleration, Mass, Velocity};
    use crate::core::coordinates::Sector;
    use crate::core::time::SimulationTime;
    use crate::physics::broadphase::{CandidatePair, CandidatePairs, KdTreeBroadphase};
    use glam::DVec3;

    #[test]
    fn test_sph_kernel_normalization() {
        let h = 1.0_f64;
        let dr = 0.005_f64;
        let mut integral = 0.0_f64;
        let n = (2.0_f64 * h / dr) as usize;
        for i in 0..n {
            let r = (i as f64 + 0.5_f64) * dr;
            integral += cubic_spline_kernel(r, h) * 4.0_f64 * PI * r.powi(2) * dr;
        }
        assert!((integral - 1.0_f64).abs() < 0.02_f64, "Kernel integral = {}", integral);
    }

    #[test]
    fn test_sph_kernel_gradient_sign() {
        let h = 1.0_f64;
        assert!(cubic_spline_kernel_gradient(0.5_f64, h) < 0.0_f64);
        assert!(cubic_spline_kernel_gradient(1.5_f64, h) < 0.0_f64);
        assert_eq!(cubic_spline_kernel_gradient(3.0_f64, h), 0.0_f64);
    }

    #[test]
    fn test_tait_eos_basic() {
        let tait = TaitEquationConfig::default();
        let b = tait.reference_density * tait.speed_of_sound.powi(2) / tait.gamma;
        let p_rest = b * (1.0_f64.powf(tait.gamma) - 1.0_f64);
        assert!(p_rest.abs() < 1e-6_f64);
        let p_high = b * ((1100.0_f64 / tait.reference_density).powf(tait.gamma) - 1.0_f64);
        assert!(p_high > 0.0_f64);
    }

    #[test]
    fn test_hydrostatic_column_forces() {
        let mut world = World::new();
        let mut uni = UniverseConfig::default();
        uni.sector_size = 1e6_f64;
        world.insert_resource(uni);
        world.insert_resource(TaitEquationConfig {
            reference_density: 1000.0_f64, speed_of_sound: 20.0_f64, gamma: 7.0_f64,
        });
        world.insert_resource(SphViscosityConfig::default());
        world.insert_resource(SimulationTime::with_dt(0.001_f64));

        let n = 20_usize;
        let (spacing, h, mass) = (0.05_f64, 0.2_f64, 100.0_f64);
        let mut ents = Vec::new();
        for i in 0..n {
            ents.push(world.spawn((
                Sector::default(), LocalPosition(DVec3::new(0.0, i as f64 * spacing, 0.0)),
                Mass(mass), Velocity(DVec3::ZERO), Force(DVec3::ZERO),
                Acceleration(DVec3::ZERO), SmoothedDensity::default(),
                Pressure::default(), SmoothingRadius(h), FluidParticle,
            )).id());
        }
        let mut pairs = Vec::new();
        for i in 0..n { for j in (i+1)..n {
            if ((j-i) as f64) * spacing < 2.0_f64 * h {
                pairs.push(CandidatePair { entity_a: ents[i], entity_b: ents[j] });
            }
        }}
        world.insert_resource(CandidatePairs(pairs));
        world.insert_resource(KdTreeBroadphase::default());

        let mut sched = Schedule::default();
        sched.add_systems(sph_density_system);
        sched.add_systems(sph_eos_system.after(sph_density_system));
        sched.add_systems(sph_pressure_force_system.after(sph_eos_system));
        sched.run(&mut world);

        let f_mid = world.get::<Force>(ents[n/2]).unwrap().0;
        assert!(f_mid.length() > 1e-6_f64, "Interior force zero: {:?}", f_mid);
        let f_edge = world.get::<Force>(ents[0]).unwrap().0;
        assert!(f_edge.length() > f_mid.length(), "Edge < interior");
    }

    /// Dam break test (qualitative, §V): a block of fluid particles
    /// under gravity should spread laterally over time. After N ticks
    /// the X-extent of the fluid should have increased.
    #[test]
    fn test_dam_break_spreading() {
        let mut world = World::new();
        let mut uni = UniverseConfig::default();
        uni.sector_size = 1e6_f64;
        world.insert_resource(uni);
        world.insert_resource(TaitEquationConfig {
            reference_density: 1000.0_f64, speed_of_sound: 20.0_f64, gamma: 7.0_f64,
        });
        world.insert_resource(SphViscosityConfig::default());
        let dt = 1e-4_f64;
        world.insert_resource(SimulationTime::with_dt(dt));

        // Create a 3×3 block of fluid on the left side (x=0..0.1, y=0..0.1)
        let spacing = 0.05_f64;
        let h = 0.08_f64;
        let mass = 50.0_f64;
        let mut ents = Vec::new();
        for ix in 0..3_usize {
            for iy in 0..3_usize {
                let pos = DVec3::new(ix as f64 * spacing, iy as f64 * spacing, 0.0);
                ents.push(world.spawn((
                    Sector::default(), LocalPosition(pos),
                    Mass(mass), Velocity(DVec3::ZERO), Force(DVec3::ZERO),
                    Acceleration(DVec3::ZERO), SmoothedDensity::default(),
                    Pressure::default(), SmoothingRadius(h), FluidParticle,
                )).id());
            }
        }

        // Record initial X-extent
        let initial_x_max: f64 = ents.iter()
            .map(|e| world.get::<LocalPosition>(*e).unwrap().0.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let initial_x_min: f64 = ents.iter()
            .map(|e| world.get::<LocalPosition>(*e).unwrap().0.x)
            .fold(f64::INFINITY, f64::min);
        let initial_extent = initial_x_max - initial_x_min;

        // Run multiple ticks: density → EOS → pressure → accel → integrate
        for _tick in 0..50 {
            // Rebuild pairs each tick (brute-force for 9 particles)
            let mut pairs = Vec::new();
            let positions: Vec<(usize, DVec3)> = ents.iter().enumerate()
                .map(|(i, e)| (i, world.get::<LocalPosition>(*e).unwrap().0))
                .collect();
            for i in 0..positions.len() {
                for j in (i+1)..positions.len() {
                    let d = (positions[i].1 - positions[j].1).length();
                    if d < 2.0_f64 * h {
                        pairs.push(CandidatePair { entity_a: ents[i], entity_b: ents[j] });
                    }
                }
            }
            world.insert_resource(CandidatePairs(pairs));
            world.insert_resource(KdTreeBroadphase::default());

            // Reset forces
            for e in &ents {
                if let Some(mut f) = world.get_mut::<Force>(*e) { f.0 = DVec3::ZERO; }
            }

            // SPH pipeline
            let mut sched = Schedule::default();
            sched.add_systems(sph_density_system);
            sched.add_systems(sph_eos_system.after(sph_density_system));
            sched.add_systems(sph_pressure_force_system.after(sph_eos_system));
            sched.run(&mut world);

            // Semi-implicit Euler: v += F/m * dt, x += v * dt
            for e in &ents {
                let f = world.get::<Force>(*e).unwrap().0;
                let m = world.get::<Mass>(*e).unwrap().0;
                let acc = f / m;
                let mut vel = world.get_mut::<Velocity>(*e).unwrap();
                vel.0 += acc * dt;
                let new_v = vel.0;
                drop(vel);
                let mut pos = world.get_mut::<LocalPosition>(*e).unwrap();
                pos.0 += new_v * dt;
            }
        }

        // After 50 ticks, X-extent should have increased (particles spread)
        let final_x_max: f64 = ents.iter()
            .map(|e| world.get::<LocalPosition>(*e).unwrap().0.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let final_x_min: f64 = ents.iter()
            .map(|e| world.get::<LocalPosition>(*e).unwrap().0.x)
            .fold(f64::INFINITY, f64::min);
        let final_extent = final_x_max - final_x_min;

        assert!(
            final_extent > initial_extent,
            "Fluid should spread: initial extent={}, final extent={}",
            initial_extent, final_extent
        );

        // All positions should remain finite
        for e in &ents {
            let p = world.get::<LocalPosition>(*e).unwrap().0;
            assert!(p.is_finite(), "Position went NaN/Inf: {:?}", p);
        }
    }
}
