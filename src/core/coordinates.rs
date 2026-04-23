//! Hierarchical coordinate system for infinite-scale simulation.
//!
//! Uses a dual representation: [`Sector`] (coarse grid, `i64`) +
//! [`LocalPosition`] (fine offset, `DVec3` in meters) to eliminate
//! floating-point drift at astronomical distances.
//!
//! With a default sector size of 1e12 m (~6.7 AU) and `i64` indices,
//! the addressable range is ±9.2×10³⁰ m — far beyond the observable
//! universe (~8.8×10²⁶ m).

use bevy_ecs::prelude::*;
use glam::{DVec3, I64Vec3};

/// Coarse-grained grid position (sector index).
///
/// Each component is a sector index along the corresponding axis.
/// The physical origin of a sector is `sector_index * sector_size` [m].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Sector(pub I64Vec3);

/// Fine-grained position offset within a sector [m].
///
/// Represents displacement from the sector's origin in meters.
/// After normalization, each component stays within `[-sector_size/2, +sector_size/2]`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct LocalPosition(pub DVec3);

impl Sector {
    /// Create a new sector at the given grid coordinates.
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self(I64Vec3::new(x, y, z))
    }

    /// Sector at the origin.
    pub const ORIGIN: Self = Self(I64Vec3::ZERO);
}

impl LocalPosition {
    /// Create a new local position [m].
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self(DVec3::new(x, y, z))
    }

    /// Zero local position.
    pub const ZERO: Self = Self(DVec3::ZERO);
}

/// Compute the displacement vector from position A to position B [m].
///
/// Takes sector boundaries into account. Returns `B - A` as a `DVec3`.
pub fn displacement(
    sector_a: &Sector,
    local_a: &LocalPosition,
    sector_b: &Sector,
    local_b: &LocalPosition,
    sector_size: f64,
) -> DVec3 {
    let sector_delta = sector_b.0 - sector_a.0;
    let sector_offset = DVec3::new(
        sector_delta.x as f64 * sector_size,
        sector_delta.y as f64 * sector_size,
        sector_delta.z as f64 * sector_size,
    );
    sector_offset + (local_b.0 - local_a.0)
}

/// Compute the squared distance between two positions across sectors [m²].
///
/// More efficient than `displacement().length()` when only magnitude is needed.
pub fn distance_squared(
    sector_a: &Sector,
    local_a: &LocalPosition,
    sector_b: &Sector,
    local_b: &LocalPosition,
    sector_size: f64,
) -> f64 {
    displacement(sector_a, local_a, sector_b, local_b, sector_size).length_squared()
}

/// Normalize a position: if `LocalPosition` exceeds `±sector_size/2` on any
/// axis, transfer the excess into the `Sector` index.
pub fn normalize_position(
    sector: &mut Sector,
    local: &mut LocalPosition,
    sector_size: f64,
) {
    let half = sector_size * 0.5_f64;

    let axes = [
        (local.0.x, 0),
        (local.0.y, 1),
        (local.0.z, 2),
    ];

    for &(value, axis) in &axes {
        if value > half {
            let sectors_to_shift = ((value + half) / sector_size).floor() as i64;
            match axis {
                0 => {
                    sector.0.x += sectors_to_shift;
                    local.0.x -= sectors_to_shift as f64 * sector_size;
                }
                1 => {
                    sector.0.y += sectors_to_shift;
                    local.0.y -= sectors_to_shift as f64 * sector_size;
                }
                2 => {
                    sector.0.z += sectors_to_shift;
                    local.0.z -= sectors_to_shift as f64 * sector_size;
                }
                _ => unreachable!(),
            }
        } else if value < -half {
            let sectors_to_shift = ((-value + half) / sector_size).floor() as i64;
            match axis {
                0 => {
                    sector.0.x -= sectors_to_shift;
                    local.0.x += sectors_to_shift as f64 * sector_size;
                }
                1 => {
                    sector.0.y -= sectors_to_shift;
                    local.0.y += sectors_to_shift as f64 * sector_size;
                }
                2 => {
                    sector.0.z -= sectors_to_shift;
                    local.0.z += sectors_to_shift as f64 * sector_size;
                }
                _ => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECTOR_SIZE: f64 = 1e12_f64;

    #[test]
    fn test_displacement_same_sector() {
        let sa = Sector::ORIGIN;
        let la = LocalPosition::new(100.0, 0.0, 0.0);
        let sb = Sector::ORIGIN;
        let lb = LocalPosition::new(300.0, 0.0, 0.0);

        let d = displacement(&sa, &la, &sb, &lb, SECTOR_SIZE);
        assert!((d.x - 200.0).abs() < 1e-10);
        assert!(d.y.abs() < 1e-10);
        assert!(d.z.abs() < 1e-10);
    }

    #[test]
    fn test_displacement_across_sectors() {
        let sa = Sector::new(0, 0, 0);
        let la = LocalPosition::new(4e11, 0.0, 0.0);
        let sb = Sector::new(1, 0, 0);
        let lb = LocalPosition::new(-4e11, 0.0, 0.0);

        let d = displacement(&sa, &la, &sb, &lb, SECTOR_SIZE);
        let expected = SECTOR_SIZE - 8e11;
        assert!((d.x - expected).abs() < 1e-2);
    }

    #[test]
    fn test_normalize_wraps_sector() {
        let mut sector = Sector::ORIGIN;
        let mut local = LocalPosition::new(1.5e12, 0.0, 0.0);
        normalize_position(&mut sector, &mut local, SECTOR_SIZE);

        assert_eq!(sector.0.x, 1);
        assert!(local.0.x.abs() < SECTOR_SIZE * 0.5 + 1.0);
    }

    #[test]
    fn test_normalize_negative_wraps() {
        let mut sector = Sector::ORIGIN;
        let mut local = LocalPosition::new(-1.5e12, 0.0, 0.0);
        normalize_position(&mut sector, &mut local, SECTOR_SIZE);

        assert_eq!(sector.0.x, -1);
        assert!(local.0.x.abs() < SECTOR_SIZE * 0.5 + 1.0);
    }

    #[test]
    fn test_distance_squared_consistency() {
        let sa = Sector::new(0, 0, 0);
        let la = LocalPosition::ZERO;
        let sb = Sector::new(1, 0, 0);
        let lb = LocalPosition::ZERO;

        let d2 = distance_squared(&sa, &la, &sb, &lb, SECTOR_SIZE);
        assert!((d2 - SECTOR_SIZE * SECTOR_SIZE).abs() < 1e6);
    }
}
