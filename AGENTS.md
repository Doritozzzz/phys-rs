# phys-rs — AGENTS.md

Universal physics engine in Rust. ECS-based, f64 precision, GPU compute.

## Build & Test

- Check: `cargo check` (prefer over build for speed)
- Build: `cargo build`
- Run: `cargo run --release`
- Test: `cargo test`
- Test single: `cargo test test_name`
- Nuclear: `cargo run --release --features nuclear`
- Bench: `cargo bench`

## First Steps for a New Session

Read these before modifying code:
1. `docs/ARCHITECTURE.md` — module structure, design constraints
2. `docs/PHYSICS_MASTER_INDEX.md` — canonical equations for all physics
3. `docs/BACKLOG.md` — pending tasks, mark completed items
4. `docs/ROADMAP.md` — phase objectives
5. `.agents/rules/*.md` — numerical safety, architecture rules
6. `AGENTS.md` (this file) — build/test/ conventions

When finishing a task, update `docs/BACKLOG.md` marking what was done.

## Project Layout

```
src/
├── main.rs           # Entry, World/Schedule setup
├── core/             # Config, time, coordinates, constants
├── components/       # ECS Component definitions (data only, no logic)
├── physics/          # Physics systems (all logic lives here)
├── gpu/              # wgpu compute pipeline and buffers
└── render/           # Visual rendering (placeholder)
```

## Code Conventions

### ECS Rules
- `bevy_ecs` standalone (NOT full Bevy engine)
- Components: `#[derive(Component)]`, data only, no methods
- Systems: plain fns with Query/Res/ResMut/Commands params
- Resources: `#[derive(Resource)]`, global singletons
- NEVER `Arc<Mutex>` in physics — use Resources
- NEVER store Entity IDs in components — use marker components

### Numerics
- `f64` / `DVec3` for ALL physics. `f32` only for GPU vertex data.
- All inverse-distance forces MUST use softening: `1 / (r² + ε²)`
- `debug_assert!(value.is_finite())` after every force/acceleration/position update
- Fixed timestep only. No variable dt in physics systems.
- NEVER standard Euler. Semi-Implicit Euler, Velocity Verlet (preferred), RK4.
- Verify overflow ordering: compute `G * m1` before `m1 * m2`.

### Determinism
- No HashMap iteration for physics data (non-deterministic)
- No RNG without `SeedableRng`
- No system clock or OS-specific behavior in physics

### Dependencies (curated, no additions without approval)
| Crate | Purpose |
|---|---|
| `glam` | DVec3, DQuat (prefer over nalgebra) |
| `nalgebra` | Matrix3, eigenvalues (only what glam lacks) |
| `parry3d-f64` | Collision shapes and narrowphase |
| `kiddo` | K-D tree broadphase |
| `wgpu` + `encase` | GPU compute + buffer marshalling |

## Schedule Order

1. `reset_forces`, `reset_torques`
2. `build_broadphase` → `broadphase_query` (K-D tree)
3. Force systems: gravity, nfw, coulomb, lorentz, geodesics, SPH (density→eos→pressure→viscosity→boundary)
4. `compute_acceleration` (F/m)
5. Integration (method-dependent):
   - SemiEuler: `semi_implicit_euler`
   - Verlet: `verlet_position` → **[second force eval]** → `verlet_velocity`
   - RK4: `rk4`
6. Post-integration: `sph_xsph` → `mhd_induction` → `divergence_clean` → relativistic → `sector_boundary`
7. `update_momentum`
8. Rotational: `compute_angular_accel` → `damping` → `rotational_integration` → `lense_thirring`
9. Collisions: `narrow_phase` → `collision_impulse`
10. Energy monitoring: kinetic → potential → drift
11. Thermodynamics: `sync_temp` → `conduction` → `radiation` → `sync_energy`

## Known Limitations (as of current state)

1. Velocity Verlet now has correct second force evaluation (fixed)
2. `normalize_position` runs via `sector_boundary_system` after integration (fixed)
3. Post-integration systems live inside each integration branch to avoid dangling deps (fixed)
4. ~76 warnings from unused components/systems — expected, will resolve as engine connects more systems
5. `barnes_hut_gravity_system` implemented but not wired to schedule yet
6. No visual render loop yet (winit/game loop pending Sandbox Phase S1)
7. No integration tests for full end-to-end simulation

## What NOT to Do

- Do NOT add new dependencies without explicit user approval
- Do NOT switch physics to f32
- Do NOT use standard Euler (energy-divergent)
- Do NOT use `unwrap()` in production code paths
- Do NOT edit `.agents/` files — they're gitignored local config
- Do NOT remove warnings about unused code — they resolve naturally as systems connect

## PR & Commit Style

- Title: `module: brief description` (e.g. `collisions: fix impulse sign for friction`)
- Run `cargo check` before committing
- Include/fix tests for changed systems
- Mark completed items in `docs/BACKLOG.md`
