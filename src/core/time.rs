//! Simulation time management.
//!
//! Provides a fixed-timestep resource and accumulator pattern
//! for deterministic physics stepping (ARCHITECTURE.md §3).

use crate::components::Velocity;
use crate::core::coordinates::{LocalPosition, Sector};
use bevy_ecs::prelude::*;

/// Fixed-timestep simulation clock.
///
/// Manages the simulation's progression through time with a
/// constant `dt` to ensure determinism. Uses an accumulator
/// pattern to convert variable frame deltas into fixed physics steps.
#[derive(Resource, Debug, Clone)]
pub struct SimulationTime {
    /// Fixed timestep duration [s].
    pub dt: f64,

    /// Number of physics ticks elapsed since simulation start.
    pub tick: u64,

    /// Total elapsed simulation time [s].
    pub elapsed: f64,

    /// Accumulator for variable-to-fixed dt conversion [s].
    pub accumulator: f64,

    /// Whether the simulation is paused.
    pub paused: bool,

    /// Speed multiplier (1.0 = realtime, 10.0 = 10× faster).
    pub speed_multiplier: f64,
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self {
            dt: 1.0_f64 / 60.0_f64,
            tick: 0,
            elapsed: 0.0_f64,
            accumulator: 0.0_f64,
            paused: true,
            speed_multiplier: 1.0_f64,
        }
    }
}

impl SimulationTime {
    /// Create a `SimulationTime` with a specific fixed timestep [s].
    pub fn with_dt(dt: f64) -> Self {
        Self {
            dt,
            ..Default::default()
        }
    }

    /// Feed a variable frame delta into the accumulator.
    ///
    /// Returns the number of fixed-step ticks to execute this frame.
    /// The accumulator retains any leftover sub-step time.
    pub fn accumulate(&mut self, frame_dt: f64) -> u32 {
        if self.paused {
            return 0;
        }
        self.accumulator += frame_dt * self.speed_multiplier;
        let steps = (self.accumulator / self.dt).floor() as u32;
        self.accumulator -= f64::from(steps) * self.dt;
        steps
    }

    /// Record that one physics tick has completed.
    pub fn advance_tick(&mut self) {
        self.tick += 1;
        self.elapsed += self.dt;
    }
}

/// Snapshot of initial entity states for R-key reset.
#[derive(Resource, Clone)]
pub struct InitialSnapshot {
    pub entities: Vec<(Entity, Sector, LocalPosition, Velocity)>,
}
