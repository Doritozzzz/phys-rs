//! NFW dark matter halo profile (PHYSICS_MASTER_INDEX §X.1).
//!
//! Implements a phantom gravitational field from a Navarro-Frenk-White
//! density profile. This adds background gravity without N-body particles,
//! producing flat galactic rotation curves.

use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::dynamics::{Force, Mass};
use crate::core::config::UniverseConfig;
use crate::core::constants::G;
use crate::core::coordinates::{LocalPosition, Sector};

// ─── Configuration ─────────────────────────────────────────────────

/// Configuration for a Navarro-Frenk-White dark matter halo (§X.1).
///
/// The NFW profile defines the density of a dark matter halo:
/// `ρ(r) = ρ₀ / [(r/Rs)(1 + r/Rs)²]`
///
/// This produces flat rotation curves at large radii, matching
/// observed galactic dynamics that Newtonian gravity alone cannot explain.
#[derive(Resource, Debug, Clone)]
pub struct NfwHaloConfig {
    /// Central characteristic density ρ₀ [kg m⁻³].
    pub rho_0: f64,

    /// Scale radius R_s [m] — transition between inner (ρ ∝ r⁻¹)
    /// and outer (ρ ∝ r⁻³) behavior.
    pub scale_radius: f64,

    /// Halo center position in local coordinates [m].
    pub center: DVec3,

    /// Whether the halo field is active.
    pub enabled: bool,
}

impl Default for NfwHaloConfig {
    fn default() -> Self {
        Self {
            // Typical Milky Way-like values
            rho_0: 1.4e-21_f64, // ~0.008 M☉/pc³ ≈ 1.4e-21 kg/m³
            scale_radius: 6.17e20_f64, // ~20 kpc ≈ 6.17e20 m
            center: DVec3::ZERO,
            enabled: false, // disabled by default — opt-in
        }
    }
}

// ─── Utility Functions ─────────────────────────────────────────────

/// NFW density at radius r (§X.1).
///
/// $$
/// \rho(r) = \frac{\rho_0}{\frac{r}{R_s}\left(1 + \frac{r}{R_s}\right)^2}
/// $$
///
/// Returns 0 for r ≤ 0.
pub fn nfw_density(r: f64, rho_0: f64, rs: f64) -> f64 {
    if r <= 0.0_f64 || rs <= 0.0_f64 {
        return 0.0_f64;
    }
    let x = r / rs; // dimensionless radius
    rho_0 / (x * (1.0_f64 + x).powi(2))
}

/// NFW enclosed mass within radius r (§X.1).
///
/// Analytical integral of the NFW profile over a sphere:
/// $$
/// M(r) = 4\pi \rho_0 R_s^3 \left[ \ln\left(1 + \frac{r}{R_s}\right)
///         - \frac{r/R_s}{1 + r/R_s} \right]
/// $$
pub fn nfw_enclosed_mass(r: f64, rho_0: f64, rs: f64) -> f64 {
    if r <= 0.0_f64 || rs <= 0.0_f64 {
        return 0.0_f64;
    }
    let x = r / rs;
    let factor = 4.0_f64 * std::f64::consts::PI * rho_0 * rs.powi(3);
    factor * ((1.0_f64 + x).ln() - x / (1.0_f64 + x))
}

/// NFW circular velocity at radius r [m s⁻¹].
///
/// `v_c(r) = √(G · M(r) / r)`
pub fn nfw_circular_velocity(r: f64, rho_0: f64, rs: f64) -> f64 {
    let m_enc = nfw_enclosed_mass(r, rho_0, rs);
    (G * m_enc / r).max(0.0_f64).sqrt()
}

// ─── ECS System ────────────────────────────────────────────────────

/// NFW dark matter halo gravity system (§X.1, task 10.5).
///
/// Applies a phantom gravitational acceleration from the dark matter
/// halo to all massive entities:
/// `F = -G · M(r) · m / r² · r̂`
///
/// This is a background field — no pairwise computation needed.
/// The halo center and parameters are defined in `NfwHaloConfig`.
pub fn nfw_gravity_system(
    mut query: Query<(&mut Force, &Mass, &Sector, &LocalPosition)>,
    config: Res<NfwHaloConfig>,
    _uni: Res<UniverseConfig>,
) {
    if !config.enabled {
        return;
    }

    let rho_0 = config.rho_0;
    let rs = config.scale_radius;
    let center = config.center;

    for (mut force, mass, _sector, pos) in &mut query {
        // Vector from halo center to entity
        let r_vec = pos.0 - center;
        let r = r_vec.length();

        if r < 1e-10_f64 {
            continue; // at center, no force
        }

        let m_enclosed = nfw_enclosed_mass(r, rho_0, rs);

        // F = -G · M(r) · m / r² in the -r̂ direction (toward center)
        let r_hat = r_vec / r;
        let f_mag = G * m_enclosed * mass.0 / (r * r);
        let f_vec = -r_hat * f_mag;

        debug_assert!(
            f_vec.is_finite(),
            "NFW gravity NaN/Inf: r={}, M_enc={:.3e}, f={:?}",
            r, m_enclosed, f_vec
        );

        force.0 += f_vec;
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// NFW density at r = Rs should equal ρ₀/4.
    #[test]
    fn test_nfw_density_at_scale_radius() {
        let rho_0 = 1.0_f64;
        let rs = 1.0_f64;
        let rho_at_rs = nfw_density(rs, rho_0, rs);
        // ρ(Rs) = ρ₀ / (1 · (1+1)²) = ρ₀ / 4
        let expected = rho_0 / 4.0_f64;
        assert!(
            (rho_at_rs - expected).abs() < 1e-12_f64,
            "ρ(Rs) should be ρ₀/4: got {}, expected {}", rho_at_rs, expected
        );
    }

    /// NFW density should follow r⁻¹ for r << Rs (inner profile).
    #[test]
    fn test_nfw_inner_profile() {
        let rho_0 = 1.0_f64;
        let rs = 100.0_f64;

        // At small r: ρ(r) ≈ ρ₀ Rs / r (since (1+r/Rs)² ≈ 1)
        let r1 = 0.01_f64;
        let r2 = 0.02_f64;
        let rho1 = nfw_density(r1, rho_0, rs);
        let rho2 = nfw_density(r2, rho_0, rs);

        // Ratio should be ≈ r2/r1 = 2 (since ρ ∝ 1/r)
        let ratio = rho1 / rho2;
        assert!(
            (ratio - 2.0_f64).abs() < 0.01_f64,
            "Inner profile should scale as 1/r: ratio={}", ratio
        );
    }

    /// NFW rotation curve should flatten at large radii.
    /// This is the observational signature of dark matter.
    #[test]
    fn test_nfw_flat_rotation_curve() {
        let rho_0 = 1e-21_f64;
        let rs = 1e20_f64;

        // Compute circular velocity at several radii beyond Rs
        let radii = vec![5.0, 10.0, 20.0, 50.0, 100.0];
        let v_circs: Vec<f64> = radii
            .iter()
            .map(|&mult| nfw_circular_velocity(mult * rs, rho_0, rs))
            .collect();

        // After Rs, the rotation curve should flatten:
        // velocity changes should be small relative to the velocity itself
        for i in 1..v_circs.len() {
            let change = (v_circs[i] - v_circs[i - 1]).abs() / v_circs[i - 1];
            assert!(
                change < 0.3_f64,
                "Rotation curve should flatten: v[{}]={:.3e}, v[{}]={:.3e}, change={:.2}%",
                i, v_circs[i], i - 1, v_circs[i - 1], change * 100.0
            );
        }

        // All velocities should be nonzero
        for (i, v) in v_circs.iter().enumerate() {
            assert!(*v > 0.0_f64, "v_circ[{}] should be > 0: {}", i, v);
        }
    }

    /// Enclosed mass should increase monotonically.
    #[test]
    fn test_nfw_enclosed_mass_monotonic() {
        let rho_0 = 1.0_f64;
        let rs = 1.0_f64;

        let radii = vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 100.0];
        let masses: Vec<f64> = radii
            .iter()
            .map(|&r| nfw_enclosed_mass(r, rho_0, rs))
            .collect();

        for i in 1..masses.len() {
            assert!(
                masses[i] > masses[i - 1],
                "M(r) should increase: M({})={:.6e} <= M({})={:.6e}",
                radii[i], masses[i], radii[i - 1], masses[i - 1]
            );
        }
    }
}
