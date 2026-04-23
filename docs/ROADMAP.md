# Project Roadmap: Universal Physics Engine

> Engine-first architecture: all physics systems are built, tested, and validated headlessly
> before any visual/interactive layer exists. The Sandbox consumes the engine as a library.

---

## ENGINE

### Phase 0: Bootstrap & Workspace Setup
- **Objective:** Establish module structure, ECS world, fixed-timestep scaffold, and physical constants.
- **Deliverables:** Compilable skeleton with `core/`, `components/`, `physics/`, `gpu/` modules. `UniverseConfig`, `SimulationTime` resources. All SI constants defined.

### Phase 1: Coordinate System & Infinite Scale
- **Objective:** Eliminate floating-point drift at astronomical distances.
- **Deliverables:** `Sector(i64)` + `LocalPosition(DVec3)` dual-coordinate system. Origin shift system. Cross-sector distance arithmetic.

### Phase 2: Core ECS Components
- **Objective:** Define all data types the engine operates on.
- **Deliverables:** Components for dynamics (`Mass`, `Velocity`, `Force`), spatial (`BoundingRadius`), material (`Temperature`, `Charge`, `Density`), rotational (`AngularVelocity`, `Orientation`, `InertiaTensor`), and identification (`BodyType`).

### Phase 3: Numerical Integration
- **Objective:** Implement time-stepping methods with proven energy conservation.
- **Deliverables:** Semi-Implicit Euler, Velocity Verlet, and RK4 integrators. Fixed-timestep accumulator. Rotational (quaternion) integration. Energy conservation validation over 100K+ steps.

### Phase 4: Gravitational Dynamics
- **Objective:** Accurate N-body gravitation from particle pairs to galactic scales.
- **Deliverables:** Brute-force O(N²) gravity with softening. Barnes-Hut O(N log N) tree approximation. Energy monitoring. Orbital mechanics utilities (vis-viva, escape velocity, Roche limit).

### Phase 5: Spatial Partitioning & Collision Detection
- **Objective:** Efficient broadphase + accurate narrowphase for millions of entities.
- **Deliverables:** K-D tree (`kiddo`) neighbor queries. `parry3d-f64` shape manifold generation. Impulse resolution with restitution and friction. Collision events. Sub-stepping for fast bodies.

### Phase 6: Rigid Body Dynamics
- **Objective:** Full rotational physics for extended bodies.
- **Deliverables:** Inertia tensor computation. Torque-driven angular acceleration. Gyroscopic precession. Contact-point torque from collisions. Linear + angular damping.

### Phase 7: Fluid Dynamics — SPH
- **Objective:** Simulate gases, liquids, and nebulae via Smoothed Particle Hydrodynamics.
- **Deliverables:** SPH density estimation, pressure gradient, viscosity, Monaghan artificial viscosity, XSPH correction. Boundary handling. Hydrostatic equilibrium validation.

### Phase 8: Thermodynamics & Heat Transfer
- **Objective:** Temperature-dependent behavior and energy exchange.
- **Deliverables:** Ideal gas law, Fourier conduction, Stefan-Boltzmann radiation, Wien's law. SPH-thermal coupling.

### Phase 9: Electromagnetism
- **Objective:** Electric and magnetic forces, plasma behavior.
- **Deliverables:** Coulomb force, Lorentz force, MHD induction equation, divergence cleaning.

### Phase 10: Relativistic & Cosmological Physics
- **Objective:** High-velocity and extreme-gravity corrections.
- **Deliverables:** Lorentz factor, relativistic momentum, Schwarzschild geodesics, NFW dark matter profiles, Lense-Thirring frame-dragging.

### Phase 11: Nuclear Astrophysics `[feature-gated]`
- **Objective:** Stellar lifecycle and nuclear processes.
- **Deliverables:** Fusion energy yield, degeneracy pressure, stellar state machine (Main Sequence → end states), Chandrasekhar limit → supernova trigger.

### Phase 12: GPU Compute Pipeline
- **Objective:** Port performance-critical systems to GPU for massive parallelism.
- **Deliverables:** wgpu compute pipeline. WGSL shaders for gravity, integration, SPH. ECS↔GPU data marshalling via `encase`. CPU fallback mode. f64 emulation on f32 hardware. Benchmarks.

---

## SANDBOX

### Phase S1: Window, Camera & Event Loop
- **Objective:** Interactive viewport into the simulation.
- **Deliverables:** `winit` window + wgpu surface. Free-fly and orbital cameras. Simulation speed controls. Entity selection. Frame timing.

### Phase S2: Rendering Pipeline
- **Objective:** Beautiful, high-performance visualization of physics state.
- **Deliverables:** Instanced sphere rendering. Temperature-based star coloring. Orbit trails. Vector overlays. SPH fluid rendering. Bloom. Gravitational lensing shader.

### Phase S3: Debug UI & Developer Tools
- **Objective:** Inspect, modify, and experiment with the simulation at runtime.
- **Deliverables:** Entity HUD. Energy graphs. Click-to-spawn. Console commands. Save/load. Performance profiler. Entity inspector.

### Phase S4: Creative Scenarios & Presets
- **Objective:** Showcase every physics capability with stunning, interactive demonstrations.
- **Deliverables:** Solar System, Binary Star, Asteroid Belt, Galaxy Formation, Star Formation Nebula, Fluid Playground, Collision Lab, Black Hole Accretion, EM Sandbox, Tidal Disruption, Supernova, Three-Body Chaos.
- **Extensibility:** Adding new scenarios requires only initial-state definitions — zero engine changes.
