---
description: Workflow to implement a new physics system (force, integrator, or material behavior) from scratch, following all project conventions and safety protocols.
---

# Workflow: Implement New Physics System

Use this workflow when adding any new physical force, integrator, or material behavior to the engine. It ensures compliance with the architecture, numerical safety, and formula correctness rules.

## Step 1: Identify the Equation

1. Open `docs/PHYSICS_MASTER_INDEX.md`.
2. Find the exact equation you are implementing. Note its section number (e.g., §IV.1 for gravitation).
3. If the equation does NOT exist in the index:
   - Add it to the correct section of `PHYSICS_MASTER_INDEX.md` with full LaTeX notation.
   - Include the equation name, all variables with their units, and any constraints or assumptions.
   - Commit this documentation update BEFORE writing any code.

## Step 2: Define Components

1. Check `src/components/` for existing components that already represent the quantities you need (e.g., `Mass`, `Velocity`, `Position`).
2. If new components are needed:
   - Create them in the appropriate file under `src/components/`.
   - Use newtype wrappers over `f64` or `DVec3`: `pub struct Charge(pub f64);`
   - Add `#[derive(Component)]`.
   - Add a `///` doc comment with units: `/// Electric charge [Coulombs (C)]`.
   - Re-export from `src/components/mod.rs`.

## Step 3: Write the System

1. Create a new file under `src/physics/` or add to an existing one.
2. Write the system function following this template:

```rust
use bevy_ecs::prelude::*;
use glam::DVec3;

/// Brief description of the physical law.
///
/// Implements [Equation Name] (PHYSICS_MASTER_INDEX §X.Y):
/// $$
/// [LaTeX equation here]
/// $$
///
/// ## Numerical Safety
/// - Softening: [describe if applicable]
/// - Precision: f64 / DVec3
pub fn my_physics_system(mut query: Query<(/* components */)>) {
    // Implementation
    
    // Finiteness validation (mandatory)
    debug_assert!(result.is_finite(), "NaN/Inf detected in [system name]");
}
```

3. **Mandatory checklist** before considering the system complete:
   - [ ] Uses `f64` and `DVec3` exclusively for physics values
   - [ ] Physical constants declared with `_f64` suffix
   - [ ] Softening applied to any inverse-distance terms
   - [ ] `debug_assert!(result.is_finite())` present
   - [ ] Doc comment references `PHYSICS_MASTER_INDEX.md` section
   - [ ] Units documented for all public items

## Step 4: Register in the Schedule

1. Open `src/main.rs` (or wherever the ECS `Schedule` is configured).
2. Add the new system to the appropriate execution stage:
   - **Force accumulation** systems run first (gravity, electromagnetic, SPH pressure).
   - **Integration** systems run after all forces are accumulated.
   - **Collision** systems run after integration.
   - **Reset** systems (zeroing forces) run at the start of the next tick.

## Step 5: Verify Conservation

Run the energy conservation workflow:

// turbo
```bash
cargo test --release -- conservation --nocapture
```

See `.agents/workflows/verify_energy.md` for detailed verification procedures and acceptance thresholds.

## Step 6: Update Documentation

1. Ensure `docs/PHYSICS_MASTER_INDEX.md` is up to date with the equation.
2. Update `docs/BACKLOG.md` to mark the corresponding task as complete: `- [x] Task X.Y`.
3. If this system completes a Roadmap phase, update `docs/ROADMAP.md`.
