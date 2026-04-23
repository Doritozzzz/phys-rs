---
trigger: always_on
glob:
description: Core architectural constraints for the phys-rs universal physics engine. Enforces ECS patterns, dependency usage, module structure, and Data-Oriented Design.
---

# Rule: Architecture & Engineering Standards

This rule enforces the foundational engineering constraints of the phys-rs project. All code must conform to these patterns. Violations will produce systems that are incompatible with the engine's GPU pipeline, ECS architecture, or determinism guarantees.

## 1. Entity Component System (ECS) — Mandatory Pattern

- **Library**: `bevy_ecs` (standalone, NOT the full Bevy engine).
- **Components** are plain data structs annotated with `#[derive(Component)]`. They MUST NOT contain logic, methods with side effects, or references to other components.
- **Systems** are plain functions that receive `Query`, `Res`, `ResMut`, `Commands`, or `EventReader`/`EventWriter` parameters. ALL logic lives in systems.
- **Resources** are global singletons annotated with `#[derive(Resource)]` for configuration and shared state (e.g., `UniverseConfig`, `SimulationTime`).
- **NEVER** store `Entity` IDs inside components to create implicit relationships. Use `bevy_ecs` relations or dedicated marker components instead.
- **NEVER** use `Arc<Mutex<T>>` or other shared-state primitives. If you need shared mutable state, it's a `Resource`.

## 2. Module Structure

The `src/` directory follows this layout:

```
src/
├── main.rs          # Entry point, App/World/Schedule setup
├── core/            # Foundational types: coordinates, config, time
│   ├── mod.rs
│   ├── coordinates.rs   # Sector(i128) + LocalPosition(DVec3)
│   ├── config.rs        # UniverseConfig resource
│   └── time.rs          # Fixed timestep resource
├── components/      # All ECS Component definitions
│   ├── mod.rs
│   ├── dynamics.rs      # Mass, Velocity, Acceleration, Force
│   ├── spatial.rs       # Position, Sector, BoundingVolume
│   └── material.rs      # Temperature, Charge, Density
├── physics/         # Physics systems (force calculators, integrators)
│   ├── mod.rs
│   ├── gravity.rs
│   ├── integrators.rs
│   └── collisions.rs
└── render/          # wgpu rendering pipeline
    ├── mod.rs
    └── pipeline.rs
```

- When creating new files, place them in the correct module according to this structure.
- Every directory MUST have a `mod.rs` that re-exports its public API.
- Physics systems go in `physics/`. Component definitions go in `components/`. Never mix them.

## 3. Dependency Usage

Each dependency in `Cargo.toml` has a specific, exclusive purpose. Do not misuse them:

| Crate | Purpose | Usage Rules |
|---|---|---|
| `bevy_ecs` | ECS framework | Components, Systems, Resources, Schedules. Do NOT use any bevy rendering. |
| `glam` | Linear algebra | `DVec3` for physics, `Vec3` only for GPU vertex data. Use `DQuat` for rotations. |
| `nalgebra` | Advanced math | Inertia tensors (`Matrix3`), eigenvalue decomposition. NOT for basic vectors. |
| `wgpu` | GPU compute & render | Compute shaders, render pipelines. All GPU interaction goes through this. |
| `encase` | GPU data mapping | `StorageBuffer` serialization for ECS → GPU transfers. Zero-copy when possible. |
| `kiddo` | K-D trees | Spatial partitioning for neighbor queries (SPH, collision broadphase). |
| `parry3d-f64` | Collision detection | Narrow-phase collision shapes and manifold generation. `f64` variant only. |
| `winit` | Windowing | Event loop and window management. No physics logic here. |

- **Do NOT add new dependencies** without explicit user approval. The dependency set is curated.
- If `glam` can do something, prefer it over `nalgebra`. Use `nalgebra` only for operations `glam` doesn't support (e.g., arbitrary matrix inversion, eigenvalues).

## 4. Data-Oriented Design (DOD)

- Components should be small and flat. Prefer `struct Velocity(pub DVec3)` over nested hierarchies.
- Avoid `Vec<T>`, `HashMap<K,V>`, or heap-allocated collections inside components. If a component needs variable-length data, reconsider the design — it probably should be multiple entities.
- Systems should iterate over components in tight loops. Avoid branching inside hot loops; use marker components and filtered queries instead.
- Data that goes to the GPU via `encase` MUST be `repr(C)` compatible and map directly to WGSL struct layouts.

## 5. Determinism

- The simulation MUST produce identical results given the same initial state and timestep, regardless of the host platform (within IEEE 754 `f64` guarantees).
- **NEVER** use `HashMap` iteration order for physics calculations (it's non-deterministic). Use `BTreeMap` or sorted `Vec` if ordered iteration is needed.
- **NEVER** use random number generators without a seeded, deterministic RNG (e.g., `rand::SeedableRng`).
- **NEVER** depend on system clock, thread scheduling, or any OS-specific behavior in physics logic.

## 6. Error Handling Philosophy

- Physics systems should use `debug_assert!` for invariant checks that catch developer mistakes (NaN, infinite values, negative masses).
- Use `Result<T, E>` for operations that can legitimately fail (file I/O, GPU resource creation, asset loading).
- **NEVER use `unwrap()`** on `Result` or `Option` in production code paths. Use `expect("descriptive message")` during prototyping, and proper error handling for anything that ships.
- Panics in physics systems are acceptable ONLY as a last-resort safety net for detecting simulation corruption (e.g., a `debug_assert!` that fires).

## 7. Documentation Standard

- Every public function, struct, and module MUST have a `///` doc comment.
- Physics systems MUST reference the corresponding equation from `docs/PHYSICS_MASTER_INDEX.md` in their doc comment. Example: `/// Implements Newton's Law of Universal Gravitation (PHYSICS_MASTER_INDEX §IV.1)`.
- Constants MUST document their units and source. Example: `/// Gravitational constant G [m³ kg⁻¹ s⁻²] (CODATA 2018)`.
