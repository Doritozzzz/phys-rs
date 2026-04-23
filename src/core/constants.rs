//! Physical constants in SI units.
//!
//! All constants are `const f64` with documented units and sources.
//! Reference: CODATA 2018 recommended values unless otherwise noted.

/// Gravitational constant G [m³ kg⁻¹ s⁻²] (CODATA 2018)
pub const G: f64 = 6.67430e-11_f64;

/// Speed of light in vacuum c [m s⁻¹] (exact, SI 2019 definition)
pub const C: f64 = 2.99792458e8_f64;

/// Boltzmann constant k_B [J K⁻¹] (exact, SI 2019 definition)
pub const K_B: f64 = 1.380649e-23_f64;

/// Stefan-Boltzmann constant σ [W m⁻² K⁻⁴] (derived from k_B, h, c)
pub const STEFAN_BOLTZMANN: f64 = 5.670374419e-8_f64;

/// Vacuum permittivity ε₀ [F m⁻¹] (CODATA 2018)
pub const EPSILON_0: f64 = 8.8541878128e-12_f64;

/// Vacuum permeability μ₀ [H m⁻¹] (derived, SI 2019)
pub const MU_0: f64 = 1.25663706212e-6_f64;

/// Planck constant h [J s] (exact, SI 2019 definition)
pub const PLANCK: f64 = 6.62607015e-34_f64;

/// Reduced Planck constant ℏ = h/(2π) [J s]
pub const HBAR: f64 = 1.054571817e-34_f64;

/// Elementary charge e [C] (exact, SI 2019 definition)
pub const ELEMENTARY_CHARGE: f64 = 1.602176634e-19_f64;

/// Coulomb constant k_e = 1/(4πε₀) [N m² C⁻²] (derived)
pub const COULOMB_CONSTANT: f64 = 8.9875517923e9_f64;

/// Wien's displacement constant b [m K] (CODATA 2018)
pub const WIEN_B: f64 = 2.897771955e-3_f64;

/// Solar mass M☉ [kg] (IAU 2015 nominal)
pub const SOLAR_MASS: f64 = 1.989e30_f64;

/// Earth mass M⊕ [kg] (IAU 2015 nominal)
pub const EARTH_MASS: f64 = 5.972e24_f64;

/// Lunar mass [kg]
pub const LUNAR_MASS: f64 = 7.342e22_f64;

/// Astronomical unit AU [m] (IAU 2012 exact)
pub const AU: f64 = 1.495978707e11_f64;

/// Light-year [m] (IAU definition)
pub const LIGHT_YEAR: f64 = 9.4607304725808e15_f64;

/// Parsec [m] (IAU 2015)
pub const PARSEC: f64 = 3.0856775814913673e16_f64;

/// Chandrasekhar mass limit [kg] (~1.4 M☉)
pub const CHANDRASEKHAR_LIMIT: f64 = 2.765e30_f64;

/// Earth-Moon distance (semi-major axis) [m]
pub const EARTH_MOON_DISTANCE: f64 = 3.844e8_f64;

/// Earth orbital velocity [m s⁻¹]
pub const EARTH_ORBITAL_VELOCITY: f64 = 2.978e4_f64;

/// Lunar orbital velocity [m s⁻¹]
pub const LUNAR_ORBITAL_VELOCITY: f64 = 1.022e3_f64;
