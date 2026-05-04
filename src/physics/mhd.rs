//! Magnetohydrodynamics (MHD) systems (PHYSICS_MASTER_INDEX §V.4, §VII.1).
//!
//! Implements:
//! - SPH-discretized magnetic induction equation (§V.4)
//! - Dedner hyperbolic/parabolic divergence cleaning for ∇·B = 0 (§VII.1)
//!
//! Reference: Price (2012) "Smoothed particle magnetohydrodynamics".

use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::dynamics::Velocity;
use crate::components::electromagnetic::{DivCleaningPsi, MagneticField};
use crate::components::sph::{FluidParticle, SmoothedDensity, SmoothingRadius};
use crate::components::dynamics::Mass;
use crate::core::config::UniverseConfig;
use crate::core::coordinates::{displacement, LocalPosition, Sector};
use crate::core::time::SimulationTime;
use crate::physics::broadphase::CandidatePairs;
use crate::physics::sph::cubic_spline_kernel_gradient;

// ─── Configuration ─────────────────────────────────────────────────

/// Configuration for magnetohydrodynamics systems.
#[derive(Resource, Debug, Clone)]
pub struct MhdConfig {
    /// Magnetic resistivity η [m² s⁻¹] (Ohmic diffusion).
    /// Controls how quickly B diffuses. Set to 0 for ideal MHD.
    pub resistivity: f64,

    /// Dedner cleaning wave speed c_h [m s⁻¹].
    /// Controls the speed at which divergence errors propagate away.
    /// Typically set to the fast magnetosonic speed.
    pub cleaning_speed: f64,

    /// Damping parameter σ_c [dimensionless] for parabolic cleaning.
    /// Controls exponential decay rate of ψ. Typical: 0.3–1.0.
    pub cleaning_damping: f64,
}

impl Default for MhdConfig {
    fn default() -> Self {
        Self {
            resistivity: 1e-4_f64,
            cleaning_speed: 100.0_f64,
            cleaning_damping: 0.5_f64,
        }
    }
}

// ─── Induction System ──────────────────────────────────────────────

/// SPH-discretized magnetic induction equation (§V.4).
///
/// $$
/// \frac{\partial \vec{B}}{\partial t} = \nabla \times (\vec{v} \times \vec{B})
///     + \eta \nabla^2 \vec{B}
/// $$
///
/// The ideal induction term is discretized using the SPH identity:
/// `dBᵢ/dt = -ρᵢ Σⱼ (mⱼ/ρⱼ) [(vⱼ - vᵢ) ⊗ (Bᵢ/ρᵢ²) + (Bⱼ/ρⱼ²) ⊗ (vⱼ - vᵢ)] · ∇Wᵢⱼ`
///
/// Simplified to the symmetric Price (2012) form for the induction term:
/// `dBᵢ/dt = (1/ρᵢ) Σⱼ mⱼ [Bᵢ(vⱼ-vᵢ) - (vⱼ-vᵢ)Bᵢ] · ∇Wᵢⱼ / ρⱼ`
///
/// The resistive diffusion term uses the SPH Laplacian:
/// `η ∇²B ≈ η Σⱼ mⱼ (Bⱼ - Bᵢ) / ρⱼ · 2|∇W| / |rᵢⱼ|`
pub fn mhd_induction_system(
    mut query: Query<(
        Entity,
        &Sector,
        &LocalPosition,
        &Velocity,
        &Mass,
        &SmoothedDensity,
        &SmoothingRadius,
        &mut MagneticField,
    ), With<FluidParticle>>,
    candidate_pairs: Res<CandidatePairs>,
    config: Res<UniverseConfig>,
    mhd: Res<MhdConfig>,
    time: Res<SimulationTime>,
) {
    let dt = time.dt;
    let eta = mhd.resistivity;

    // Collect B deltas to avoid aliasing during iteration
    let mut deltas: Vec<(Entity, DVec3)> = Vec::new();

    for pair in &candidate_pairs.0 {
        let (e1, e2) = (pair.entity_a, pair.entity_b);

        if let Ok([
            (_, s1, p1, v1, m1, d1, h1, b1),
            (_, s2, p2, v2, m2, d2, h2, b2),
        ]) = query.get_many_mut([e1, e2])
        {
            let r_vec = displacement(s1, p1, s2, p2, config.sector_size);
            let r = r_vec.length();
            let h_avg = 0.5_f64 * (h1.0 + h2.0);

            if r > 1e-10_f64 && r < 2.0_f64 * h_avg {
                let r_hat = r_vec / r;
                let dw_dr = cubic_spline_kernel_gradient(r, h_avg);
                let grad_w = r_hat * dw_dr;

                let v_ij = v1.0 - v2.0; // v_i - v_j

                // ── Ideal induction term (§V.4) ──
                // Symmetric form: for particle i,
                //   dB_i/dt += (m_j / (ρ_i ρ_j)) * [B_i (v_ij · ∇W) - v_ij (B_i · ∇W)]
                // This preserves ∇·B = 0 in the ideal limit.

                let v_dot_gradw = v_ij.dot(grad_w);
                let b_dot_gradw_1 = b1.0.dot(grad_w);
                let b_dot_gradw_2 = b2.0.dot(grad_w);

                let factor_12 = m2.0 / (d1.0 * d2.0);
                let factor_21 = m1.0 / (d2.0 * d1.0);

                let db1_ideal = factor_12 * (b1.0 * v_dot_gradw - v_ij * b_dot_gradw_1);
                let db2_ideal = factor_21 * (b2.0 * (-v_dot_gradw) - (-v_ij) * b_dot_gradw_2);

                // ── Resistive diffusion term ──
                // η ∇²B ≈ 2η Σⱼ (mⱼ/ρⱼ) (Bⱼ - Bᵢ) (rᵢⱼ · ∇Wᵢⱼ) / (|rᵢⱼ|² + 0.01h²)
                let b_diff = b2.0 - b1.0;
                let r_dot_gradw = r_vec.dot(grad_w);
                let denom = r.powi(2) + 0.01_f64 * h_avg.powi(2);

                let db1_resist = 2.0_f64 * eta * (m2.0 / d2.0) * b_diff * r_dot_gradw / denom;
                let db2_resist = 2.0_f64 * eta * (m1.0 / d1.0) * (-b_diff) * (-r_dot_gradw) / denom;

                let db1_total = (db1_ideal + db1_resist) * dt;
                let db2_total = (db2_ideal + db2_resist) * dt;

                deltas.push((e1, db1_total));
                deltas.push((e2, db2_total));
            }
        }
    }

    // Apply accumulated deltas
    for (entity, db) in deltas {
        if let Ok((_, _, _, _, _, _, _, mut b_field)) = query.get_mut(entity) {
            b_field.0 += db;
            debug_assert!(
                b_field.0.is_finite(),
                "MHD induction produced NaN/Inf on entity {:?}", entity
            );
        }
    }
}

// ─── Divergence Cleaning ───────────────────────────────────────────

/// Dedner hyperbolic/parabolic divergence cleaning system (§VII.1).
///
/// Enforces `∇·B = 0` by evolving a scalar correction field ψ:
///
/// ```text
/// ∂B/∂t += -∇ψ
/// ∂ψ/∂t = -c_h² (∇·B) - (c_h² σ_c / h) ψ
/// ```
///
/// where `c_h` is the cleaning wave speed and `σ_c` is the damping
/// parameter. This propagates divergence errors out of the domain
/// at speed `c_h` while damping them exponentially.
///
/// SPH discretization:
/// - `∇·B ≈ Σⱼ (mⱼ/ρⱼ) (Bⱼ - Bᵢ) · ∇Wᵢⱼ`
/// - `∇ψ  ≈ Σⱼ (mⱼ/ρⱼ) (ψⱼ - ψᵢ) ∇Wᵢⱼ`
pub fn divergence_cleaning_system(
    mut query: Query<(
        Entity,
        &Sector,
        &LocalPosition,
        &Mass,
        &SmoothedDensity,
        &SmoothingRadius,
        &mut MagneticField,
        &mut DivCleaningPsi,
    ), With<FluidParticle>>,
    candidate_pairs: Res<CandidatePairs>,
    config: Res<UniverseConfig>,
    mhd: Res<MhdConfig>,
    time: Res<SimulationTime>,
) {
    let dt = time.dt;
    let ch = mhd.cleaning_speed;
    let ch2 = ch * ch;
    let sigma = mhd.cleaning_damping;

    // Collect corrections
    let mut b_corrections: Vec<(Entity, DVec3)> = Vec::new();
    let mut psi_corrections: Vec<(Entity, f64)> = Vec::new();

    for pair in &candidate_pairs.0 {
        let (e1, e2) = (pair.entity_a, pair.entity_b);

        if let Ok([
            (_, s1, p1, m1, d1, h1, b1, psi1),
            (_, s2, p2, m2, d2, h2, b2, psi2),
        ]) = query.get_many_mut([e1, e2])
        {
            let r_vec = displacement(s1, p1, s2, p2, config.sector_size);
            let r = r_vec.length();
            let h_avg = 0.5_f64 * (h1.0 + h2.0);

            if r > 1e-10_f64 && r < 2.0_f64 * h_avg {
                let r_hat = r_vec / r;
                let dw_dr = cubic_spline_kernel_gradient(r, h_avg);
                let grad_w = r_hat * dw_dr;

                // ∇·B contribution (for ψ evolution)
                let b_diff = b2.0 - b1.0;
                let div_b_contrib_1 = (m2.0 / d2.0) * b_diff.dot(grad_w);
                let div_b_contrib_2 = (m1.0 / d1.0) * (-b_diff).dot(-grad_w);

                // ∇ψ contribution (for B correction)
                let psi_diff = psi2.0 - psi1.0;
                let grad_psi_1 = (m2.0 / d2.0) * psi_diff * grad_w;
                let grad_psi_2 = (m1.0 / d1.0) * (-psi_diff) * (-grad_w);

                // B correction: ∂B/∂t += -∇ψ
                b_corrections.push((e1, -grad_psi_1 * dt));
                b_corrections.push((e2, -grad_psi_2 * dt));

                // ψ correction: ∂ψ/∂t += -c_h² ∇·B
                psi_corrections.push((e1, -ch2 * div_b_contrib_1 * dt));
                psi_corrections.push((e2, -ch2 * div_b_contrib_2 * dt));
            }
        }
    }

    // Apply B corrections
    for (entity, db) in b_corrections {
        if let Ok((_, _, _, _, _, _, mut b_field, _)) = query.get_mut(entity) {
            b_field.0 += db;
            debug_assert!(b_field.0.is_finite(), "Div cleaning B NaN on {:?}", entity);
        }
    }

    // Apply ψ corrections from SPH divergence computation
    for (entity, dpsi) in psi_corrections {
        if let Ok((_, _, _, _, _, _, _, mut psi)) = query.get_mut(entity) {
            psi.0 += dpsi;
            debug_assert!(psi.0.is_finite(), "Div cleaning ψ NaN on {:?}", entity);
        }
    }

    // Parabolic damping: applied to ALL entities, not just those with pairs.
    // ψ *= exp(-σ · c_h · dt / h)
    // This exponential decay drives ψ → 0 regardless of neighbor interactions.
    for (_, _, _, _, _, h, _, mut psi) in &mut query {
        let decay = (-sigma * ch * dt / h.0).exp();
        psi.0 *= decay;
        debug_assert!(psi.0.is_finite(), "Div cleaning ψ damping NaN");
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::dynamics::{Force, Velocity};
    use crate::components::sph::{FluidParticle, Pressure, SmoothedDensity, SmoothingRadius};
    use crate::core::coordinates::Sector;
    use crate::core::time::SimulationTime;
    use crate::physics::broadphase::{CandidatePair, CandidatePairs, KdTreeBroadphase};
    use glam::DVec3;

    /// Verify that divergence cleaning damps ψ toward zero.
    #[test]
    fn test_div_cleaning_damps_psi() {
        let mut world = World::new();
        let mut uni = UniverseConfig::default();
        uni.sector_size = 1e6_f64;
        world.insert_resource(uni);
        world.insert_resource(SimulationTime::with_dt(0.001_f64));
        world.insert_resource(MhdConfig::default());
        world.insert_resource(CandidatePairs(Vec::new()));
        world.insert_resource(KdTreeBroadphase::default());

        let e = world.spawn((
            Sector::default(),
            LocalPosition(DVec3::ZERO),
            Mass(1.0_f64),
            Velocity(DVec3::ZERO),
            Force(DVec3::ZERO),
            SmoothedDensity(1000.0_f64),
            SmoothingRadius(0.1_f64),
            MagneticField(DVec3::new(1.0, 0.0, 0.0)),
            DivCleaningPsi(10.0_f64), // large initial ψ
            FluidParticle,
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(divergence_cleaning_system);

        for _ in 0..100 {
            schedule.run(&mut world);
        }

        let psi = world.get::<DivCleaningPsi>(e).expect("missing").0;
        assert!(
            psi.abs() < 1.0_f64,
            "ψ should have damped significantly: got {}", psi
        );
    }

    /// Uniform B field should remain unchanged under induction
    /// (no velocity gradients → no curl → dB/dt = 0).
    #[test]
    fn test_uniform_b_unchanged() {
        let mut world = World::new();
        let mut uni = UniverseConfig::default();
        uni.sector_size = 1e6_f64;
        world.insert_resource(uni);
        world.insert_resource(SimulationTime::with_dt(0.001_f64));
        world.insert_resource(MhdConfig { resistivity: 0.0_f64, ..Default::default() });

        let b_init = DVec3::new(0.0, 0.0, 1.0);
        let spacing = 0.05_f64;
        let h = 0.1_f64;
        let mut ents = Vec::new();

        // 3 particles with uniform velocity and uniform B
        for i in 0..3_usize {
            ents.push(world.spawn((
                Sector::default(),
                LocalPosition(DVec3::new(i as f64 * spacing, 0.0, 0.0)),
                Mass(1.0_f64),
                Velocity(DVec3::new(1.0, 0.0, 0.0)), // uniform velocity
                Force(DVec3::ZERO),
                SmoothedDensity(1000.0_f64),
                SmoothingRadius(h),
                MagneticField(b_init),
                DivCleaningPsi(0.0_f64),
                FluidParticle,
            )).id());
        }

        let mut pairs = Vec::new();
        for i in 0..3_usize {
            for j in (i + 1)..3_usize {
                pairs.push(CandidatePair { entity_a: ents[i], entity_b: ents[j] });
            }
        }
        world.insert_resource(CandidatePairs(pairs));
        world.insert_resource(KdTreeBroadphase::default());

        let mut schedule = Schedule::default();
        schedule.add_systems(mhd_induction_system);

        for _ in 0..100 {
            schedule.run(&mut world);
        }

        for e in &ents {
            let b = world.get::<MagneticField>(*e).expect("missing").0;
            let drift = (b - b_init).length();
            assert!(
                drift < 1e-6_f64,
                "Uniform B should be unchanged: got {:?}, drift={}", b, drift
            );
        }
    }
}
