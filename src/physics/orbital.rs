use crate::core::constants::{C, G};

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
