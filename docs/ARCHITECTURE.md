# Engineering Architecture

## 1. Core Paradigm

- **Pattern:** Entity Component System (ECS).
- **Library:** `bevy_ecs` (standalone — NOT the full Bevy engine).
- **Constraint:** Logic must be strictly separated from data. Systems process Components; Entities are unique identifiers.
- **Engine/Sandbox Separation:** The physics engine (`core/`, `components/`, `physics/`, `gpu/`) produces zero visual output. The sandbox (`render/`) consumes the engine as a library for interactive visualization.

## 2. Mathematical Standards

- **Precision:** Double Precision Floating Point (`f64`) for all physical calculations.
- **Library:** `glam` (`DVec3`, `DQuat`). `nalgebra` only for advanced operations (`Matrix3`, eigenvalues).
- **Units:** International System of Units (SI).
  - Distance: Meters (m)
  - Mass: Kilograms (kg)
  - Time: Seconds (s)
  - Force: Newtons (N)
  - Energy: Joules (J)
  - Temperature: Kelvin (K)

## 3. Simulation Integrity

- **Time Management:** Fixed Timestep execution (accumulator pattern).
- **Determinism:** The simulation state must be reproducible given the same initial conditions and delta-time.
- **Memory:** Data-Oriented Design (DOD) — flat components, tight iteration, cache-friendly layouts.
- **Numerical Safety:** Softened force denominators, `debug_assert!` finiteness checks, ordered iteration (no `HashMap`).

## 4. Module Structure

```
src/
├── main.rs              # Entry point: World, Schedule, timestep loop
├── core/                # Foundational types and configuration
│   ├── mod.rs
│   ├── config.rs        # UniverseConfig resource (G, ε, dt, θ, integrator)
│   ├── time.rs          # SimulationTime resource (fixed dt, tick counter, accumulator)
│   ├── coordinates.rs   # Sector(i64) + LocalPosition(DVec3) dual-coordinate system
│   └── constants.rs     # Physical constants (G, c, k_B, σ, ε₀, μ₀, h, e)
├── components/          # All ECS Component definitions (data only, no logic)
│   ├── mod.rs
│   ├── dynamics.rs      # Mass, Velocity, Acceleration, Force, PreviousAcceleration, LinearMomentum
│   ├── spatial.rs       # WorldPosition, BoundingRadius
│   ├── rotational.rs    # AngularVelocity, Orientation, Torque, InertiaTensor
│   ├── material.rs      # Temperature, Charge, Density, Luminosity, Opacity
│   ├── thermal.rs       # InternalEnergy, HeatCapacity, ThermalConductivity
│   ├── electromagnetic.rs # ElectricField, MagneticField
│   ├── sph.rs           # SmoothedDensity, Pressure, SmoothingRadius
│   └── identifiers.rs   # EntityName, BodyType
├── physics/             # All physics systems (logic only, no data definitions)
│   ├── mod.rs
│   ├── forces.rs        # Force/Torque accumulator reset
│   ├── gravity.rs       # N-body brute-force gravitation
│   ├── gravity_tree.rs  # Barnes-Hut octree approximation
│   ├── integrators.rs   # Semi-Implicit Euler, Velocity Verlet, RK4
│   ├── origin.rs        # Origin shift & sector boundary crossing
│   ├── broadphase.rs    # K-D tree spatial partitioning (kiddo)
│   ├── collisions.rs    # Narrow-phase manifolds + impulse resolution (parry3d-f64)
│   ├── rigid_body.rs    # Inertia tensors, torque, angular dynamics
│   ├── sph.rs           # SPH density, pressure, viscosity, artificial viscosity
│   ├── thermodynamics.rs # Heat transfer, radiation, gas law
│   └── electrostatics.rs # Coulomb, Lorentz, MHD
├── gpu/                 # GPU compute pipeline (wgpu + encase)
│   ├── mod.rs
│   ├── buffers.rs       # StorageBuffer wrappers for ECS↔GPU marshalling
│   └── pipeline.rs      # Compute pipeline abstraction, shader dispatch
└── render/              # SANDBOX: Visual layer (winit + wgpu rendering)
    ├── mod.rs
    └── pipeline.rs      # Render pipeline, camera, instanced draws
```

## 5. Dependency Map

| Crate | Purpose | Module |
|---|---|---|
| `bevy_ecs` | ECS framework (Components, Systems, Resources, Schedules) | all |
| `glam` | `DVec3`/`DQuat` for physics, `Vec3` for GPU vertex data only | `core/`, `components/`, `physics/` |
| `nalgebra` | Inertia tensors (`Matrix3`), eigenvalues, matrix inversion | `physics/` |
| `wgpu` | GPU compute shaders + render pipeline | `gpu/`, `render/` |
| `encase` | `StorageBuffer` serialization for ECS→GPU transfer | `gpu/` |
| `kiddo` | K-D tree spatial partitioning for neighbor queries | `physics/broadphase` |
| `parry3d-f64` | Collision shapes, contact manifold generation (f64 only) | `physics/collisions` |
| `winit` | Window creation and OS event loop | `render/` |

## 6. Feature Gates

| Feature Flag | Modules Enabled | Purpose |
|---|---|---|
| `nuclear` | Nuclear astrophysics systems | Stellar lifecycle, fusion, degeneracy pressure |
| *(default)* | Everything except `nuclear` | Core engine + sandbox |
