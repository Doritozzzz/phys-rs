//! Thermodynamics & heat transfer systems (PHYSICS_MASTER_INDEX §VI).
//!
//! Implements:
//! - Temperature ↔ internal energy coupling via heat capacity
//! - Ideal gas pressure from temperature (§VI.1)
//! - Fourier's law heat conduction between neighbors (§VI.2)
//! - Stefan-Boltzmann radiative cooling/heating (§VI.2)
//! - Wien's displacement law for peak emission wavelength (§VI.2)
//! - SPH-thermal coupling (temperature-driven EOS)

use bevy_ecs::prelude::*;

use crate::components::dynamics::Mass;
use crate::components::material::Temperature;
use crate::components::thermal::{HeatCapacity, InternalEnergy, ThermalConductivity};
use crate::core::config::UniverseConfig;
use crate::core::constants::{K_B, STEFAN_BOLTZMANN, WIEN_B};
use crate::core::coordinates::{displacement, LocalPosition, Sector};
use crate::core::time::SimulationTime;
use crate::physics::broadphase::CandidatePairs;

// ─── Thermal Marker ────────────────────────────────────────────────

/// Marker component for entities participating in thermal simulation.
#[derive(bevy_ecs::component::Component, Debug, Clone, Copy, Default)]
pub struct ThermalBody;

// ─── Configuration ─────────────────────────────────────────────────

/// Configuration for radiative heat transfer.
#[derive(Resource, Debug, Clone)]
pub struct RadiationConfig {
    /// Emissivity ε ∈ [0, 1]. 1.0 = perfect blackbody.
    pub emissivity: f64,
    /// Effective radiating surface area [m²] per body.
    /// In a full engine this would be per-entity; for now it's global.
    pub default_surface_area: f64,
    /// Background temperature [K] (cosmic microwave background = 2.725 K).
    pub background_temperature: f64,
}

impl Default for RadiationConfig {
    fn default() -> Self {
        Self {
            emissivity: 1.0_f64,
            default_surface_area: 1.0_f64,
            background_temperature: 2.725_f64,
        }
    }
}

/// Configuration for SPH-thermal coupling.
#[derive(Resource, Debug, Clone)]
pub struct ThermalConductionConfig {
    /// Smoothing length for thermal conduction [m].
    /// Typically matches the SPH smoothing radius.
    pub smoothing_length: f64,
}

impl Default for ThermalConductionConfig {
    fn default() -> Self {
        Self {
            smoothing_length: 0.15_f64,
        }
    }
}

// ─── Systems ───────────────────────────────────────────────────────

/// Synchronizes temperature from internal energy: `T = U / (m · cᵥ)`.
///
/// Runs at the start of the thermal pipeline to ensure `Temperature`
/// reflects the current `InternalEnergy` state.
/// (PHYSICS_MASTER_INDEX §VI.1)
pub fn sync_temperature_system(
    mut query: Query<(&InternalEnergy, &Mass, &HeatCapacity, &mut Temperature), With<ThermalBody>>,
) {
    for (energy, mass, cv, mut temp) in &mut query {
        if mass.0 > 0.0_f64 && cv.0 > 0.0_f64 {
            temp.0 = energy.0 / (mass.0 * cv.0);
            debug_assert!(temp.0.is_finite(), "Temperature is NaN/Inf");
        }
    }
}

/// Updates internal energy from temperature: `U = m · cᵥ · T`.
///
/// Inverse of `sync_temperature_system`. Runs after thermal processes
/// modify temperature directly.
/// (PHYSICS_MASTER_INDEX §VI.1)
pub fn sync_internal_energy_system(
    mut query: Query<(&mut InternalEnergy, &Mass, &HeatCapacity, &Temperature), With<ThermalBody>>,
) {
    for (mut energy, mass, cv, temp) in &mut query {
        energy.0 = mass.0 * cv.0 * temp.0;
        debug_assert!(energy.0.is_finite(), "InternalEnergy is NaN/Inf");
    }
}

/// Heat conduction via Fourier's law between neighboring bodies (§VI.2).
///
/// For each candidate pair within the smoothing length:
/// `dQ/dt = -k_avg · A_contact · (T_j - T_i) / d`
///
/// where `k_avg` is the harmonic mean of thermal conductivities,
/// `A_contact` is an effective contact area (approximated as 1/d²),
/// and `d` is the distance between bodies.
///
/// Energy is transferred symmetrically: hot loses, cold gains.
pub fn heat_conduction_system(
    mut query: Query<(
        Entity, &Sector, &LocalPosition, &Mass, &HeatCapacity,
        &ThermalConductivity, &mut Temperature,
    ), With<ThermalBody>>,
    candidate_pairs: Res<CandidatePairs>,
    config: Res<UniverseConfig>,
    conduction: Res<ThermalConductionConfig>,
    time: Res<SimulationTime>,
) {
    let dt = time.dt;
    let h = conduction.smoothing_length;

    // Collect temperature deltas to avoid aliasing
    let mut deltas: Vec<(Entity, f64)> = Vec::new();

    for pair in &candidate_pairs.0 {
        if let Ok([
            (_, s1, p1, m1, cv1, k1, t1),
            (_, s2, p2, m2, cv2, k2, t2),
        ]) = query.get_many_mut([pair.entity_a, pair.entity_b])
        {
            let r_vec = displacement(s1, p1, s2, p2, config.sector_size);
            let d = r_vec.length();

            if d < h && d > 1e-12_f64 {
                // Harmonic mean of conductivities
                let k_avg = if k1.0 > 0.0_f64 && k2.0 > 0.0_f64 {
                    2.0_f64 * k1.0 * k2.0 / (k1.0 + k2.0)
                } else {
                    0.0_f64
                };

                if k_avg > 0.0_f64 {
                    // Heat flux: dQ = k_avg * (T2 - T1) / d * dt
                    // Effective contact scaling: 1/d gives units [W/m * 1/m * K * s = J/K * K = J]
                    let dq = k_avg * (t2.0 - t1.0) / d * dt;

                    // dT = dQ / (m * cv)
                    if m1.0 * cv1.0 > 0.0_f64 {
                        deltas.push((pair.entity_a, dq / (m1.0 * cv1.0)));
                    }
                    if m2.0 * cv2.0 > 0.0_f64 {
                        deltas.push((pair.entity_b, -dq / (m2.0 * cv2.0)));
                    }
                }
            }
        }
    }

    // Apply deltas
    for (entity, dt_val) in deltas {
        if let Ok((_, _, _, _, _, _, mut temp)) = query.get_mut(entity) {
            temp.0 += dt_val;
            // Clamp to physical minimum (absolute zero)
            if temp.0 < 0.0_f64 {
                temp.0 = 0.0_f64;
            }
            debug_assert!(temp.0.is_finite(), "Conduction produced NaN temperature");
        }
    }
}

/// Radiative cooling/heating via Stefan-Boltzmann law (§VI.2).
///
/// Each body radiates power `P = ε·σ·A·T⁴` and absorbs from the
/// background at `P_bg = ε·σ·A·T_bg⁴`. Net heat loss:
/// `dT/dt = -ε·σ·A·(T⁴ - T_bg⁴) / (m·cᵥ)`
///
/// This produces exponential-like cooling for hot bodies.
pub fn radiative_cooling_system(
    mut query: Query<(&Mass, &HeatCapacity, &mut Temperature), With<ThermalBody>>,
    radiation: Res<RadiationConfig>,
    time: Res<SimulationTime>,
) {
    let dt = time.dt;
    let eps = radiation.emissivity;
    let area = radiation.default_surface_area;
    let t_bg = radiation.background_temperature;
    let t_bg4 = t_bg.powi(4);

    for (mass, cv, mut temp) in &mut query {
        if mass.0 * cv.0 <= 0.0_f64 {
            continue;
        }

        let t4 = temp.0.powi(4);
        // Net radiated power [W] = ε·σ·A·(T⁴ - T_bg⁴)
        let net_power = eps * STEFAN_BOLTZMANN * area * (t4 - t_bg4);
        // dT = -P·dt / (m·cᵥ)   [negative = cooling when T > T_bg]
        let dt_val = -net_power * dt / (mass.0 * cv.0);

        temp.0 += dt_val;
        if temp.0 < 0.0_f64 {
            temp.0 = 0.0_f64;
        }
        debug_assert!(temp.0.is_finite(), "Radiation produced NaN temperature");
    }
}

/// Wien's displacement law: computes the peak emission wavelength (§VI.2).
///
/// `λ_max = b / T` where b = 2.8977719e-3 [m·K].
///
/// Returns the wavelength in meters. Useful for rendering (star color).
/// Returns `f64::INFINITY` for T ≈ 0 (no emission).
#[inline]
pub fn wien_peak_wavelength(temperature: f64) -> f64 {
    if temperature > 1e-10_f64 {
        WIEN_B / temperature
    } else {
        f64::INFINITY
    }
}

/// Ideal gas pressure from temperature (§VI.1).
///
/// `P = ρ · R_specific · T` where `R_specific = k_B / m_particle`.
///
/// For SPH-thermal coupling: this can replace or supplement the
/// Tait EOS when simulating hot gases (stellar envelopes, nebulae).
#[inline]
pub fn ideal_gas_pressure(density: f64, temperature: f64, particle_mass_kg: f64) -> f64 {
    if particle_mass_kg > 0.0_f64 {
        density * (K_B / particle_mass_kg) * temperature
    } else {
        0.0_f64
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::dynamics::Mass;
    use crate::components::material::Temperature;
    use crate::components::thermal::{HeatCapacity, InternalEnergy, ThermalConductivity};
    use crate::core::coordinates::Sector;
    use crate::core::time::SimulationTime;
    use crate::physics::broadphase::{CandidatePair, CandidatePairs, KdTreeBroadphase};
    use glam::DVec3;

    /// Isolated body radiative cooling (§VI.2).
    ///
    /// A hot body (10000 K) should cool exponentially via Stefan-Boltzmann
    /// radiation. After many ticks, T should decrease monotonically
    /// and remain well above background.
    #[test]
    fn test_radiative_cooling() {
        let mut world = World::new();
        world.insert_resource(SimulationTime::with_dt(0.001_f64)); // 1ms steps
        world.insert_resource(RadiationConfig {
            emissivity: 1.0_f64,
            default_surface_area: 1.0_f64,
            background_temperature: 2.725_f64,
        });

        let initial_temp = 10000.0_f64;
        let mass = 1.0_f64;
        let cv = 1000.0_f64; // J/(kg·K)

        let e = world.spawn((
            Mass(mass),
            HeatCapacity(cv),
            Temperature(initial_temp),
            ThermalBody,
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(radiative_cooling_system);

        let mut prev_temp = initial_temp;
        for _tick in 0..200 {
            schedule.run(&mut world);
            let t = world.get::<Temperature>(e).unwrap().0;

            // Temperature should decrease monotonically
            assert!(
                t <= prev_temp,
                "Temperature should decrease: prev={}, now={}", prev_temp, t
            );
            // Should remain non-negative (0K from clamping is valid)
            assert!(t >= 0.0_f64, "Temperature went negative: {}", t);

            prev_temp = t;
        }

        // After 200 ms, should have cooled somewhat
        let final_temp = world.get::<Temperature>(e).unwrap().0;
        assert!(
            final_temp < initial_temp,
            "Should have cooled: initial={}, final={}", initial_temp, final_temp
        );
    }

    /// Two bodies in thermal contact (§VI.2).
    ///
    /// One hot (1000 K) and one cold (100 K) body at close range.
    /// After many conduction ticks, they should approach thermal
    /// equilibrium: both converge to ~550 K (mean, equal masses).
    #[test]
    fn test_thermal_conduction_equilibrium() {
        let mut world = World::new();

        let mut uni = UniverseConfig::default();
        uni.sector_size = 1e6_f64;
        world.insert_resource(uni);
        world.insert_resource(SimulationTime::with_dt(0.01_f64));
        world.insert_resource(ThermalConductionConfig {
            smoothing_length: 10.0_f64,
        });
        world.insert_resource(CandidatePairs(Vec::new()));
        world.insert_resource(KdTreeBroadphase::default());

        let mass = 1.0_f64;
        let cv = 100.0_f64;
        let k = 1000.0_f64; // high conductivity for fast equilibration

        let t_hot = 1000.0_f64;
        let t_cold = 100.0_f64;
        let t_expected = (t_hot + t_cold) / 2.0_f64; // 550 K for equal masses

        let e_hot = world.spawn((
            Sector::default(),
            LocalPosition(DVec3::new(0.0, 0.0, 0.0)),
            Mass(mass),
            HeatCapacity(cv),
            ThermalConductivity(k),
            Temperature(t_hot),
            ThermalBody,
        )).id();

        let e_cold = world.spawn((
            Sector::default(),
            LocalPosition(DVec3::new(1.0, 0.0, 0.0)), // 1m apart
            Mass(mass),
            HeatCapacity(cv),
            ThermalConductivity(k),
            Temperature(t_cold),
            ThermalBody,
        )).id();

        // Build pair
        world.insert_resource(CandidatePairs(vec![
            CandidatePair { entity_a: e_hot, entity_b: e_cold },
        ]));

        let mut schedule = Schedule::default();
        schedule.add_systems(heat_conduction_system);

        // Run for many ticks
        for _ in 0..5000 {
            schedule.run(&mut world);
        }

        let t1 = world.get::<Temperature>(e_hot).unwrap().0;
        let t2 = world.get::<Temperature>(e_cold).unwrap().0;

        // Both should be close to the mean temperature
        assert!(
            (t1 - t_expected).abs() < 10.0_f64,
            "Hot body should approach equilibrium: got {}, expected ~{}", t1, t_expected
        );
        assert!(
            (t2 - t_expected).abs() < 10.0_f64,
            "Cold body should approach equilibrium: got {}, expected ~{}", t2, t_expected
        );

        // Heat should have flowed from hot to cold
        assert!(t1 < t_hot, "Hot body should have cooled: {} -> {}", t_hot, t1);
        assert!(t2 > t_cold, "Cold body should have warmed: {} -> {}", t_cold, t2);

        // Total energy should be conserved (equal masses/cv)
        let total_initial = t_hot + t_cold;
        let total_final = t1 + t2;
        assert!(
            (total_final - total_initial).abs() < 1.0_f64,
            "Energy not conserved: initial sum={}, final sum={}", total_initial, total_final
        );
    }

    /// Wien's displacement law: verify known values.
    #[test]
    fn test_wien_displacement() {
        // Sun surface ~5778 K → λ_max ≈ 502 nm (green)
        let lambda_sun = wien_peak_wavelength(5778.0_f64);
        assert!(
            (lambda_sun - 5.014e-7_f64).abs() < 1e-8_f64,
            "Sun peak wavelength should be ~501nm, got {}m", lambda_sun
        );

        // Very cold → large wavelength
        let lambda_cold = wien_peak_wavelength(3.0_f64);
        assert!(lambda_cold > 1e-4_f64, "Cold body should emit in far IR");

        // T=0 → infinity
        let lambda_zero = wien_peak_wavelength(0.0_f64);
        assert!(lambda_zero.is_infinite(), "T=0 should give infinite wavelength");
    }

    /// Ideal gas pressure sanity check.
    #[test]
    fn test_ideal_gas_pressure() {
        // Hydrogen gas: m_particle ≈ 1.67e-27 kg (proton)
        let m_h = 1.6726219e-27_f64;
        let rho = 1.0_f64;     // 1 kg/m³
        let temp = 300.0_f64;  // room temperature

        let p = ideal_gas_pressure(rho, temp, m_h);
        // P = ρ·(k_B/m_H)·T ≈ 1.0 * 8.26e3 * 300 ≈ 2.48e6 Pa
        assert!(p > 1e6_f64 && p < 1e7_f64, "H gas pressure at 300K, 1kg/m³ should be ~2.5MPa, got {}", p);

        // Zero temperature → zero pressure
        assert_eq!(ideal_gas_pressure(1.0_f64, 0.0_f64, m_h), 0.0_f64);
    }
}
