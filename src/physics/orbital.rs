use crate::core::constants::{C, G};
use glam::DVec3;

/// Computes points on a Keplerian orbit from state vector (r, v) and central mass.
///
/// Returns positions **relative to the central body** (focus), sampled evenly in
/// eccentric anomaly. For bound orbits only (elliptical, e < 1). Returns empty
/// `Vec` for hyperbolic/unbound orbits or invalid input.
///
/// # Parameters
/// - `r`: Position of orbiting body relative to central body [m].
/// - `v`: Velocity of orbiting body [m s⁻¹].
/// - `m_central`: Mass of central body [kg].
/// - `n`: Number of sample points (≥ 3).
pub fn kepler_orbit_points(r: DVec3, v: DVec3, m_central: f64, n: usize) -> Vec<DVec3> {
    use std::f64::consts::PI;

    if m_central <= 0.0 || n < 3 {
        return Vec::new();
    }

    let mu = G * m_central;
    let r_mag = r.length();
    let v2 = v.length_squared();

    if r_mag < 1e-30 || v2 < 1e-60 {
        return Vec::new();
    }

    // Specific angular momentum
    let h = r.cross(v);
    let h_mag = h.length();

    if h_mag < 1e-30 {
        return Vec::new(); // Radial trajectory, can't determine orbit plane
    }

    // Eccentricity vector: e = (v × h) / μ - r̂
    let e_vec = (v.cross(h)) / mu - r / r_mag;
    let e = e_vec.length();

    // Specific orbital energy: ε = v²/2 - μ/r
    let energy = v2 * 0.5 - mu / r_mag;

    // Semi-major axis from vis-viva: 1/a = 2/r - v²/μ → a = -μ / (2ε)
    let a = if energy.abs() < 1e-30 {
        r_mag // Nearly parabolic, approximate
    } else {
        -mu / (2.0 * energy)
    };

    if a <= 0.0 || e >= 1.0 {
        return Vec::new(); // Hyperbolic or unbound
    }

    // Orbital plane basis (perifocal frame)
    let h_hat = h.normalize();
    let p_hat = if e > 1e-12 {
        e_vec.normalize() // Points to periapsis
    } else {
        (r / r_mag).normalize() // Circular: use radial direction
    };
    let q_hat = h_hat.cross(p_hat).normalize();

    let b = a * (1.0 - e * e).sqrt(); // Semi-minor axis

    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let ecc_anomaly = 2.0 * PI * i as f64 / n as f64;
        let (sin_e, cos_e) = ecc_anomaly.sin_cos();
        // Parametric ellipse relative to focus:
        // x = a(cos E - e), y = b sin E
        let x = a * (cos_e - e);
        let y = b * sin_e;
        points.push(p_hat * x + q_hat * y);
    }

    points
}

/// Calculates velocity magnitude from Vis-Viva equation (PHYSICS_MASTER_INDEX §IV.2).
/// 
/// `v = sqrt(G * M * (2/r - 1/a))`
///
/// Returns NaN if the orbit is hyperbolic and parameters are invalid for the square root, 
/// though typically valid for bound orbits.
pub fn vis_viva_velocity(mass_central: f64, r: f64, semi_major_axis: f64) -> f64 {
    (G * mass_central * (2.0 / r - 1.0 / semi_major_axis)).max(0.0).sqrt()
}

/// Calculates escape velocity (PHYSICS_MASTER_INDEX §IV.2).
/// 
/// `v_e = sqrt(2 * G * M / r)`
pub fn escape_velocity(mass_central: f64, r: f64) -> f64 {
    (2.0 * G * mass_central / r).max(0.0).sqrt()
}

/// Calculates the Roche limit for a rigid satellite (PHYSICS_MASTER_INDEX §IV.3).
///
/// `d = 2.44 * R_M * (rho_M / rho_m)^(1/3)`
pub fn roche_limit(radius_primary: f64, density_primary: f64, density_satellite: f64) -> f64 {
    2.44 * radius_primary * (density_primary / density_satellite).cbrt()
}

/// Schwarzschild radius (PHYSICS_MASTER_INDEX §IV.3).
/// 
/// `r_s = 2 * G * M / c^2`
pub fn schwarzschild_radius(mass: f64) -> f64 {
    2.0 * G * mass / (C * C)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_vis_viva_circular_orbit() {
        // For circular orbit, r = a. v = sqrt(GM/r)
        let m = 1.989e30; // Sun
        let r = 1.496e11; // 1 AU
        let v = vis_viva_velocity(m, r, r);
        let expected = (G * m / r).sqrt();
        assert!((v - expected).abs() < 1.0);
    }

    #[test]
    fn test_escape_velocity() {
        let earth_mass = 5.972e24;
        let earth_radius = 6371e3;
        let v_esc = escape_velocity(earth_mass, earth_radius);
        // ~11186 m/s
        assert!((v_esc - 11186.0).abs() < 10.0);
    }
}
