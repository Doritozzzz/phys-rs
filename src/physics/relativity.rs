//! Special relativity systems (PHYSICS_MASTER_INDEX §VIII).
//!
//! Implements:
//! - Lorentz factor computation with near-c clamping (§VIII, task 10.1)
//! - Relativistic momentum `p = γm₀v` (§VIII, task 10.2)
//! - Relativistic Doppler shift for rendering (§VIII, task 10.3)

use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::dynamics::{
    DopplerShift, LorentzFactor, Mass, RelativisticMomentum, Velocity,
};
use crate::core::constants::C;
use crate::core::coordinates::LocalPosition;

/// Speed of light squared, precomputed for frequent use.
const C2: f64 = C * C;

/// Minimum β² threshold below which γ is approximated as 1.0.
/// At v = 0.001c, γ ≈ 1.0000005 — indistinguishable from 1.0.
const BETA2_THRESHOLD: f64 = 1e-6_f64;

/// Maximum β² allowed (prevents division by zero at v = c).
/// Corresponds to γ_max ≈ 10_000.
const BETA2_MAX: f64 = 1.0_f64 - 1e-8_f64;

// ─── Utility Functions ─────────────────────────────────────────────

/// Compute the Lorentz factor from velocity squared (§VIII).
///
/// $$
/// \gamma = \frac{1}{\sqrt{1 - v^2/c^2}}
/// $$
///
/// ## Clamping
/// - For `v² < BETA2_THRESHOLD · c²` (~0.001c): returns 1.0 (no correction)
/// - For `v² > BETA2_MAX · c²`: clamps to prevent `γ → ∞`
pub fn lorentz_factor(v_squared: f64) -> f64 {
    let beta2 = v_squared / C2;

    if beta2 < BETA2_THRESHOLD {
        return 1.0_f64;
    }

    let clamped_beta2 = beta2.min(BETA2_MAX);
    let gamma = 1.0_f64 / (1.0_f64 - clamped_beta2).sqrt();

    debug_assert!(gamma.is_finite(), "Lorentz factor NaN/Inf: β²={}", beta2);
    gamma
}

/// Compute the relativistic Doppler shift (§VIII).
///
/// $$
/// \frac{f_{obs}}{f_{emit}} = \sqrt{\frac{1 - v_r/c}{1 + v_r/c}}
/// $$
///
/// - `v_radial > 0` means receding → redshift (ratio < 1)
/// - `v_radial < 0` means approaching → blueshift (ratio > 1)
///
/// Clamps `|v_r|` to `(1-ε)c` to prevent division by zero.
pub fn doppler_shift(v_radial: f64) -> f64 {
    let v_r = v_radial.clamp(-(1.0_f64 - 1e-8_f64) * C, (1.0_f64 - 1e-8_f64) * C);
    let ratio = ((1.0_f64 - v_r / C) / (1.0_f64 + v_r / C)).sqrt();
    debug_assert!(ratio.is_finite(), "Doppler shift NaN/Inf: v_r={}", v_radial);
    ratio
}

// ─── ECS Systems ───────────────────────────────────────────────────

/// Updates the cached Lorentz factor for each relativistic entity (task 10.1).
///
/// Runs after velocity integration to reflect the current speed.
pub fn compute_lorentz_factor_system(
    mut query: Query<(&mut LorentzFactor, &Velocity)>,
) {
    for (mut gamma, vel) in &mut query {
        gamma.0 = lorentz_factor(vel.0.length_squared());
    }
}

/// Computes relativistic momentum `p = γm₀v` (task 10.2).
///
/// For entities with `v > 0.1c`, this also effectively limits
/// acceleration by tracking the relativistic inertia. The momentum
/// diverges as v → c, which the integrator can use to prevent
/// superluminal velocities.
pub fn relativistic_momentum_system(
    mut query: Query<(
        &mut RelativisticMomentum,
        &LorentzFactor,
        &Mass,
        &Velocity,
    )>,
) {
    for (mut rel_p, gamma, mass, vel) in &mut query {
        rel_p.0 = vel.0 * (gamma.0 * mass.0);
        debug_assert!(
            rel_p.0.is_finite(),
            "Relativistic momentum NaN/Inf: γ={}, m={}, v={:?}",
            gamma.0, mass.0, vel.0
        );
    }
}

/// Computes the relativistic Doppler shift for each entity (task 10.3).
///
/// Uses the radial velocity component (toward/away from origin) to
/// determine the frequency shift. The observer is assumed at the origin.
pub fn compute_doppler_shift_system(
    mut query_observer: Query<(&mut DopplerShift, &Velocity, &LocalPosition)>,
) {
    for (mut ds, vel, pos) in query_observer.iter_mut() {
        let r = pos.0;
        let r_mag = r.length();
        if r_mag > 1e-10_f64 {
            let r_hat = r / r_mag;
            let v_radial = vel.0.dot(r_hat);
            ds.0 = doppler_shift(v_radial);
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Lorentz factor should be 1.0 for slow particles (task 10.1).
    #[test]
    fn test_lorentz_factor_slow() {
        // 1000 m/s — utterly non-relativistic
        let v = 1000.0_f64;
        let gamma = lorentz_factor(v * v);
        assert!(
            (gamma - 1.0_f64).abs() < 1e-10_f64,
            "γ should be ~1.0 for v << c: got {}", gamma
        );
    }

    /// Lorentz factor grows without bound as v → c (task 10.8).
    #[test]
    fn test_lorentz_factor_diverges() {
        let gammas: Vec<f64> = [0.1, 0.5, 0.9, 0.99, 0.999, 0.9999]
            .into_iter()
            .map(|beta| {
                let v = beta * C;
                lorentz_factor(v * v)
            })
            .collect();

        // Each γ should be strictly larger than the previous
        for i in 1..gammas.len() {
            assert!(
                gammas[i] > gammas[i - 1],
                "γ should increase monotonically: γ[{}]={} <= γ[{}]={}",
                i, gammas[i], i - 1, gammas[i - 1]
            );
        }

        // At v = 0.9999c, γ should be > 70
        assert!(
            gammas[5] > 70.0_f64,
            "γ at v=0.9999c should be > 70: got {}", gammas[5]
        );
    }

    /// Relativistic momentum p = γmv diverges as v → c (task 10.8).
    #[test]
    fn test_relativistic_momentum_diverges() {
        let m = 1.0_f64; // 1 kg

        let betas = [0.1_f64, 0.5, 0.9, 0.99, 0.999];
        let momenta: Vec<f64> = betas
            .iter()
            .map(|&beta| {
                let v = beta * C;
                let gamma = lorentz_factor(v * v);
                gamma * m * v
            })
            .collect();

        // Momentum should increase faster than linearly
        for i in 1..momenta.len() {
            assert!(momenta[i] > momenta[i - 1]);
        }

        // At v = 0.999c, p >> m*c (classical momentum would be just m*v)
        let classical_p_at_999 = m * 0.999_f64 * C;
        assert!(
            momenta[4] > classical_p_at_999 * 10.0_f64,
            "Relativistic p should be >> classical at v=0.999c: rel={:.3e}, class={:.3e}",
            momenta[4], classical_p_at_999
        );
    }

    /// Doppler shift: approaching → blueshift (ratio > 1).
    #[test]
    fn test_doppler_blueshift() {
        // v_r = -0.5c (approaching)
        let ds = doppler_shift(-0.5_f64 * C);
        assert!(
            ds > 1.0_f64,
            "Approaching should blueshift: got {}", ds
        );
        // Analytical: sqrt((1+0.5)/(1-0.5)) = sqrt(3) ≈ 1.732
        let expected = 3.0_f64.sqrt();
        assert!(
            (ds - expected).abs() < 0.001_f64,
            "Doppler at v=-0.5c: expected {:.4}, got {:.4}", expected, ds
        );
    }

    /// Doppler shift: receding → redshift (ratio < 1).
    #[test]
    fn test_doppler_redshift() {
        // v_r = +0.5c (receding)
        let ds = doppler_shift(0.5_f64 * C);
        assert!(
            ds < 1.0_f64,
            "Receding should redshift: got {}", ds
        );
        // Analytical: sqrt((1-0.5)/(1+0.5)) = sqrt(1/3) ≈ 0.577
        let expected = (1.0_f64 / 3.0_f64).sqrt();
        assert!(
            (ds - expected).abs() < 0.001_f64,
            "Doppler at v=+0.5c: expected {:.4}, got {:.4}", expected, ds
        );
    }
}
