//! Origin shift and sector boundary crossing systems.
//!
//! Ensures `LocalPosition` stays within valid bounds by transferring
//! excess displacement into the `Sector` grid index.

use bevy_ecs::prelude::*;

use crate::core::coordinates::{normalize_position, LocalPosition, Sector};
use crate::core::UniverseConfig;

/// Normalizes all entity positions: transfers excess `LocalPosition`
/// into `Sector` indices when the local offset exceeds `±sector_size/2`.
///
/// Should run AFTER integration updates positions but BEFORE any
/// system that reads positions for force calculations.
pub fn sector_boundary_system(
    config: Res<UniverseConfig>,
    mut query: Query<(&mut Sector, &mut LocalPosition)>,
) {
    let sector_size = config.sector_size;
    for (mut sector, mut local) in &mut query {
        normalize_position(&mut sector, &mut local, sector_size);
    }
}
