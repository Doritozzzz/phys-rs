---
description: Workflow to verify energy conservation after implementing or modifying any physics force or integrator. Ensures the simulation does not gain or lose energy over time.
---

# Workflow: Verify Energy Conservation

Use this workflow after implementing or modifying ANY force calculation or numerical integrator in `src/physics/`. Energy conservation is the primary indicator that a physics system is correctly implemented.

## Prerequisites

- The force or integrator system must be registered in the ECS schedule.
- At least two bodies with known masses and initial conditions must be spawnable.

## Step 1: Create a Deterministic Test Scenario

Set up a two-body gravitational orbit (Earth-Moon is the standard reference):

```rust
// Test constants (SI units)
const EARTH_MASS: f64 = 5.972e24;    // kg
const MOON_MASS: f64 = 7.342e22;     // kg
const ORBITAL_RADIUS: f64 = 3.844e8; // m (average Earth-Moon distance)
const ORBITAL_VELOCITY: f64 = 1022.0; // m/s (Moon's orbital velocity)
const DT: f64 = 60.0;                // 1-minute timestep
const TOTAL_STEPS: u64 = 10_000_000; // ~19 years of simulation
```

Spawn the two bodies:
- Earth at origin `(0, 0, 0)` with zero velocity.
- Moon at `(ORBITAL_RADIUS, 0, 0)` with velocity `(0, ORBITAL_VELOCITY, 0)`.

## Step 2: Compute Initial Total Energy

// turbo
```bash
cargo test --release -- energy_conservation --nocapture
```

Calculate and store the initial total mechanical energy:

$$E_{total} = E_{kinetic} + E_{potential} = \sum_i \frac{1}{2} m_i |\vec{v}_i|^2 - \sum_{i<j} G \frac{m_i m_j}{r_{ij}}$$

## Step 3: Run the Simulation

Execute the simulation for `TOTAL_STEPS` iterations using the fixed timestep `DT`. Every 100,000 steps, compute the current total energy and record the relative drift:

$$\delta E = \frac{|E_{current} - E_{initial}|}{|E_{initial}|}$$

## Step 4: Evaluate Results

| Integrator | Acceptable Drift ($\delta E$) after $10^7$ steps | Action if exceeded |
|---|---|---|
| Semi-Implicit Euler | $< 10^{-3}$ (0.1%) | Expected for Euler; acceptable for prototyping |
| Velocity Verlet | $< 10^{-6}$ (0.0001%) | Bug in implementation — check acceleration recomputation |
| RK4 | $< 10^{-9}$ | Bug in stage computation — verify $k_1$ through $k_4$ |

## Step 5: Diagnose Failures

If energy is NOT conserved:

1. **Energy increasing monotonically**: The integrator is non-symplectic. Check if you're using standard Euler instead of Semi-Implicit Euler (velocity must update BEFORE position).
2. **Energy oscillating wildly**: The timestep `DT` is too large for the forces involved. Reduce `DT` or implement adaptive sub-stepping.
3. **Energy suddenly jumps to NaN/Inf**: A singularity occurred. Check that the softening factor ($\epsilon$) is applied in the force calculation.
4. **Energy slowly drifting in one direction (Verlet)**: The half-step acceleration recalculation is missing or using stale data. Review the Velocity Verlet implementation against §I.2 of `PHYSICS_MASTER_INDEX.md`.

## Step 6: Log Results

Print a summary to stdout:

```
[ENERGY CONSERVATION TEST]
  Integrator: Velocity Verlet
  Steps:      10,000,000
  dt:         60.0 s
  E_initial:  -3.8281e+28 J
  E_final:    -3.8281e+28 J
  Drift:      2.41e-08 (0.0000024%)
  RESULT:     PASS ✓
```
