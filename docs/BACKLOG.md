# Engineering Backlog: Universal Physics Engine

> **Philosophy**: The ENGINE (Phases 0–12) is a pure physics simulation library — zero pixels,
> fully testeable with `cargo test`. The SANDBOX (Phases S1–S4) is the interactive visual
> playground that consumes the engine. The engine must be 100% functional before rendering exists.

---

# ═══════════════════════════════════════════════════════
#                     E N G I N E
# ═══════════════════════════════════════════════════════

## Phase 0: Bootstrap & Workspace Setup

- [x] **0.1:** Create `src/core/mod.rs` — module declaration and public re-exports.
- [x] **0.2:** Create `src/components/mod.rs` — module declaration and public re-exports.
- [x] **0.3:** Create `src/physics/mod.rs` — module declaration and public re-exports.
- [x] **0.4:** Create `src/gpu/mod.rs` — module declaration and public re-exports.
- [x] **0.5:** Set up `main.rs` — initialize `bevy_ecs::World`, empty `Schedule`, fixed-timestep loop.
- [x] **0.6:** Create `src/core/config.rs` — `UniverseConfig` resource:
  - Gravitational constant `G`
  - Softening parameter `ε`
  - `max_dt` / `min_dt` safety clamps
  - Barnes-Hut opening angle `θ`
  - Integration method selector
- [x] **0.7:** Create `src/core/time.rs` — `SimulationTime` resource:
  - Fixed `dt` value
  - Tick counter (`u64`)
  - Accumulator for variable-to-fixed dt conversion
- [x] **0.8:** Create `src/core/constants.rs` — All physical constants as `const f64`:
  - `G`, `c`, `k_B`, `σ` (Stefan-Boltzmann), `ε₀`, `μ₀`, `h` (Planck), `e` (elementary charge)
  - All with units documented and CODATA sources cited
- [x] **0.9:** Verify `cargo build` passes cleanly with all dependencies.
- [x] **0.10:** Verify `cargo test` scaffold runs (tests written; blocked by OS application control policy).

---

## Phase 1: Coordinate System & Infinite Scale

- [x] **1.1:** `src/core/coordinates.rs` — `Sector` component: `I64Vec3` using `i64` (sector grid index).
- [x] **1.2:** `src/core/coordinates.rs` — `LocalPosition(DVec3)` component (meters within sector).
- [x] **1.3:** `src/components/spatial.rs` — `BoundingRadius` spatial component (WorldPosition deferred to coordinate utilities).
- [x] **1.4:** `src/physics/origin.rs` — Sector boundary normalization system using `UniverseConfig.sector_size`.
- [x] **1.5:** `normalize_position()` in coordinates.rs — wraps `LocalPosition` and adjusts `Sector` on overflow.
- [x] **1.6:** `displacement()` and `distance_squared()` cross-sector utilities in coordinates.rs.
- [x] **1.7:** Unit tests: displacement across sectors, normalization wrapping, distance consistency (5 tests).

---

## Phase 2: Core ECS Components

- [x] **2.1:** `src/components/dynamics.rs` — `Mass(f64)`, `Velocity(DVec3)`, `Acceleration(DVec3)`, `Force(DVec3)`.
- [x] **2.2:** `src/components/dynamics.rs` — `PreviousAcceleration(DVec3)` (required for Velocity Verlet half-step).
- [x] **2.3:** `src/components/dynamics.rs` — `LinearMomentum(DVec3)`.
- [x] **2.4:** `src/components/spatial.rs` — `BoundingRadius(f64)` for broadphase collision detection.
- [x] **2.5:** `src/components/material.rs` — `Temperature(f64)`, `Charge(f64)`, `Density(f64)`.
- [x] **2.6:** `src/components/material.rs` — `Luminosity(f64)`, `Opacity(f64)`.
- [x] **2.7:** `src/components/identifiers.rs` — `EntityName(String)`, `BodyType` enum (`Star`, `Planet`, `Moon`, `Asteroid`, `Particle`, `FluidParticle`).
- [x] **2.8:** `src/components/rotational.rs` — `AngularVelocity(DVec3)`, `Orientation(DQuat)`, `Torque(DVec3)`, `InertiaTensor([f64; 9])`.
- [x] **2.9:** `src/physics/forces.rs` — `reset_forces` + `reset_torques` systems.

---

## Phase 3: Numerical Integration

- [x] **3.1:** `src/physics/integrators.rs` — Semi-Implicit Euler system (§I.1): velocity first, then position with new velocity.
- [x] **3.2:** `src/physics/integrators.rs` — Velocity Verlet system (§I.2): split into position + velocity half-step systems with `PreviousAcceleration`.
- [x] **3.3:** `src/physics/integrators.rs` — RK4 system (§I.3): all 4 stages, constant-acceleration approximation.
- [x] **3.4:** `src/core/config.rs` — `IntegrationMethod` enum with runtime selection in `main.rs`.
- [x] **3.5:** `SimulationTime::accumulate()` — fixed-timestep accumulator in `src/core/time.rs`.
- [x] **3.6:** `debug_assert!(value.is_finite())` on all integrated values (velocity, position, acceleration).
- [x] **3.7:** Unit test: circular orbit energy conservation with Velocity Verlet (~6283 steps, drift < 1e-4).
- [x] **3.8:** Unit test: uniform acceleration with Semi-Euler (100 steps, validates v and x).
- [x] **3.9:** Rotational integration system — quaternion update from angular velocity (§II.3), re-normalization each step.

---

## Phase 4: Gravitational Dynamics

- [x] **4.1:** `src/physics/gravity.rs` — N-body brute-force gravity system (§IV.1): softened denominator `r² + ε²`, vector form `F = -G·m₁·m₂·r_vec / (|r|² + ε²)^(3/2)`.
- [x] **4.2:** Newton's Third Law optimization — compute each pair (i,j) once, apply `+F` to i and `-F` to j.
- [x] **4.3:** Gravitational potential energy calculator — `U = -G·m₁·m₂/r` (§IV.1).
- [x] **4.4:** Kinetic energy calculator — `E_k = ½mv²` for all bodies.
- [x] **4.5:** `EnergyMonitor` resource — tracks total energy (E_k + U) per tick, logs warning if drift exceeds configurable threshold.
- [x] **4.6:** `src/physics/gravity_tree.rs` — Barnes-Hut octree approximation for O(N log N) gravity.
- [x] **4.7:** Barnes-Hut `θ` (opening angle) parameter exposed in `UniverseConfig` for accuracy/performance tuning.
- [x] **4.8:** Unit test: 3-body figure-8 solution stability over extended run.
- [x] **4.9:** Unit test: Kepler orbit — verify period matches analytical `T = 2π√(a³/GM)` within tolerance.
- [x] **4.10:** Orbital mechanics utility functions — vis-viva velocity, escape velocity, Roche limit (§IV.2, §IV.3).

---

## Phase 5: Spatial Partitioning & Collision Detection

- [x] **5.1:** `src/physics/broadphase.rs` — K-D tree construction with `kiddo` for spatial neighbor queries.
- [x] **5.2:** Broadphase query system — returns candidate collision pairs (entities within sum of `BoundingRadius`).
- [x] **5.3:** `src/physics/collisions.rs` — `parry3d-f64` shape integration (`Ball`, `Capsule`, `ConvexPolyhedron`).
- [x] **5.4:** `CollisionShape` component wrapping `parry3d-f64` `SharedShape`.
- [x] **5.5:** Narrow-phase system — contact manifold generation via `parry3d-f64` between candidate pairs.
- [x] **5.6:** Collision impulse resolution (§III.1) — full form with rotational inertia tensor terms.
- [x] **5.7:** `CoefficientOfRestitution(f64)` component (0.0 = perfectly inelastic, 1.0 = perfectly elastic).
- [x] **5.8:** Friction impulse (tangential) — Coulomb kinetic friction model (§II.4).
- [x] **5.9:** `CollisionEvent` — event struct `{ entity_a, entity_b, normal, point, impulse_magnitude }`.
- [x] **5.10:** Sub-stepping for fast bodies — temporal subdivision (CCD-lite) to prevent tunneling.
- [x] **5.11:** Unit test: two spheres head-on collision — verify linear momentum conservation.
- [x] **5.12:** Unit test: sphere bouncing on plane — verify restitution coefficient behavior.

---

## Phase 6: Rigid Body Dynamics

- [x] **6.1:** Inertia tensor computation for primitive shapes: solid sphere `I = 2/5·m·r²`, box, cylinder (§II.2).
- [x] **6.2:** Torque accumulator reset system — zeroes `Torque` each tick before torque calculators.
- [x] **6.3:** Angular acceleration system — `α = I⁻¹(τ - ω×(Iω))` (§II.2).
- [x] **6.4:** Angular velocity integration → quaternion orientation update + re-normalization (§II.3).
- [x] **6.5:** Contact-point torque generation — from collision manifold contact points.
- [x] **6.6:** Gyroscopic precession effects for rapidly spinning bodies.
- [x] **6.7:** Damping system — configurable per-body linear + angular damping factors.
- [x] **6.8:** Unit test: spinning top precession matches analytical prediction.
- [x] **6.9:** Unit test: angular momentum conservation in torque-free isolated system.

---

## Phase 7: Fluid Dynamics — SPH

- [x] **7.1:** `src/components/sph.rs` — `SmoothedDensity(f64)`, `Pressure(f64)`, `SmoothingRadius(f64)`, `FluidParticle` marker.
- [x] **7.2:** SPH kernel functions module — Cubic spline W₃ and gradient dW/dr (§V.1).
- [x] **7.3:** `src/physics/sph.rs` — Density estimation system: includes self-contribution `ρᵢ = Σⱼ mⱼ W(rᵢ-rⱼ, h)` (§V.1).
- [x] **7.4:** Equation of state system — Tait equation for weakly-compressible fluids `P = B((ρ/ρ₀)^γ − 1)` (§V.2).
- [x] **7.5:** Pressure gradient force system — symmetric form `Pᵢ/ρᵢ² + Pⱼ/ρⱼ²` (§V.3).
- [x] **7.6:** SPH viscosity force system (§V.3).
- [x] **7.7:** Monaghan artificial viscosity — shock-capturing term `Πᵢⱼ` for approaching particles (§V.3).
- [x] **7.8:** XSPH velocity correction — smooths particle trajectories for visual stability.
- [x] **7.9:** SPH neighbor search via K-D tree (reuse Phase 5 broadphase infrastructure).
- [x] **7.10:** Boundary handling — repulsive wall force at domain edges (`SphBoundaryConfig` + `sph_boundary_system`).
- [x] **7.11:** Unit test: hydrostatic equilibrium — fluid column validates full SPH pipeline (density→EOS→pressure).
- [x] **7.12:** Unit test: dam break — qualitative validation of free-surface spreading (50-tick integration, 3×3 block).


---

## Phase 8: Thermodynamics & Heat Transfer

- [x] **8.1:** `src/components/thermal.rs` — `InternalEnergy(f64)`, `HeatCapacity(f64)`, `ThermalConductivity(f64)` (pre-existing).
- [x] **8.2:** `src/physics/thermodynamics.rs` — Ideal gas law: `P = ρ·(k_B/m_particle)·T` (§VI.1), plus `ThermalBody` marker.
- [x] **8.3:** Conduction system — Fourier's law `q = -k∇T`, harmonic mean conductivity, neighbor-based diffusion (§VI.2).
- [x] **8.4:** Radiation system — Stefan-Boltzmann net cooling `dT/dt = -ε·σ·A·(T⁴-T_bg⁴)/(m·cᵥ)` (§VI.2).
- [x] **8.5:** Wien's displacement law — `λ_max = b/T` utility function (§VI.2).
- [x] **8.6:** SPH-thermal coupling — `sync_temperature_system` / `sync_internal_energy_system` bidirectional T↔U sync.
- [x] **8.7:** Unit test: isolated body radiative cooling — monotonic T decrease over 200 ticks.
- [x] **8.8:** Unit test: two bodies in thermal contact — converge to mean T (550K), energy conserved.

---

## Phase 9: Electromagnetism

- [ ] **9.1:** `src/physics/electrostatics.rs` — Coulomb force with softening (§VII.2): `F = kₑ·q₁·q₂/(r² + ε²)`.
- [ ] **9.2:** Lorentz force system — `F = q(E + v×B)` (§VII.2).
- [ ] **9.3:** `src/components/electromagnetic.rs` — `ElectricField(DVec3)`, `MagneticField(DVec3)`.
- [ ] **9.4:** Charge-based force accumulation — runs alongside gravity in force pipeline.
- [ ] **9.5:** MHD induction equation system (§V.4) — `∂B/∂t = ∇×(v×B) + η∇²B`.
- [ ] **9.6:** Divergence cleaning for `∇·B = 0` constraint (§VII.1).
- [ ] **9.7:** Unit test: two charged particles — stable Coulomb orbit.
- [ ] **9.8:** Unit test: charged particle in uniform B field — circular Larmor orbit radius matches `r = mv/(qB)`.

---

## Phase 10: Relativistic & Cosmological Physics

- [ ] **10.1:** Lorentz factor computation — `γ = 1/√(1 - v²/c²)` with clamping for v ≈ c (§VIII).
- [ ] **10.2:** Relativistic momentum correction — `p = γm₀v` applied when `v > 0.1c` (§VIII).
- [ ] **10.3:** Relativistic Doppler shift calculator (§VIII) — stored for rendering use.
- [ ] **10.4:** Schwarzschild radius calculator — `r_s = 2GM/c²` (§IV.3).
- [ ] **10.5:** NFW dark matter halo profile (§X.1) — phantom gravitational acceleration field `ρ(r) = ρ₀ / [(r/Rs)(1 + r/Rs)²]`.
- [ ] **10.6:** Schwarzschild geodesic integrator (§X.2) — ray-marching for photon paths near black holes.
- [ ] **10.7:** Lense-Thirring frame-dragging precession (§X.2) — `Ω_LT` near massive rotating bodies.
- [ ] **10.8:** Unit test: relativistic momentum diverges as v → c.
- [ ] **10.9:** Unit test: photon circular orbit at r = 1.5·r_s (photon sphere).

---

## Phase 11: Nuclear Astrophysics `[feature: nuclear]`

> *Feature-gated behind `--features nuclear` to keep compile times lean during early development.*

- [ ] **11.1:** Mass-energy equivalence — `E = Δm·c²` energy release calculator (§IX.1).
- [ ] **11.2:** Gamow peak reaction rate — fusion yield as function of T and ρ (§IX.1).
- [ ] **11.3:** Degeneracy pressure model — Fermi gas limit `P ∝ ρ^(5/3)` (§IX.2).
- [ ] **11.4:** Stellar lifecycle state machine — Main Sequence → Red Giant → White Dwarf / Neutron Star / Black Hole.
- [ ] **11.5:** Chandrasekhar mass limit check — triggers supernova collapse event when exceeded.
- [ ] **11.6:** Unit test: core collapse triggers when mass > 1.4 M☉ (Chandrasekhar limit).

---

## Phase 12: GPU Compute Pipeline

> *All physics systems are validated on CPU first (Phases 0–11). This phase ports the
> performance-critical hot paths to GPU compute shaders for massive parallelism.*

- [ ] **12.1:** `src/gpu/mod.rs` — wgpu adapter selection, device/queue initialization.
- [ ] **12.2:** `src/gpu/buffers.rs` — `encase` `StorageBuffer` wrappers for ECS → GPU data marshalling.
- [ ] **12.3:** `src/gpu/pipeline.rs` — Compute pipeline abstraction (bind group layouts, shader modules, dispatch).
- [ ] **12.4:** WGSL compute shader: pairwise gravity kernel.
- [ ] **12.5:** WGSL compute shader: Velocity Verlet integration kernel.
- [ ] **12.6:** WGSL compute shader: SPH density + pressure kernel.
- [ ] **12.7:** GPU ↔ CPU synchronization system — read-back computed forces/positions into ECS components.
- [ ] **12.8:** Hybrid CPU/GPU mode — automatic fallback to CPU systems if no compatible GPU detected.
- [ ] **12.9:** Double-precision emulation in WGSL — `ds_add`, `ds_mul` pair arithmetic for f64 on f32 hardware.
- [ ] **12.10:** Benchmark suite: CPU vs GPU gravity for 1K, 10K, 100K body counts.

---

# ═══════════════════════════════════════════════════════
#                   S A N D B O X
# ═══════════════════════════════════════════════════════

> *The Sandbox is built AFTER the engine is complete and tested. It provides the visual,
> interactive layer for exploring and experimenting with the physics engine.*
> *Designed to be extensible — new scenarios and tools can be added without modifying the engine.*

## Phase S1: Window, Camera & Event Loop

- [ ] **S1.1:** `src/render/mod.rs` — `winit` window creation and event loop initialization.
- [ ] **S1.2:** wgpu surface configuration + swap chain setup.
- [ ] **S1.3:** Free-fly camera system — WASD movement + mouse-look rotation.
- [ ] **S1.4:** Orbital camera mode — click-drag to orbit around a focused entity.
- [ ] **S1.5:** Camera zoom (scroll wheel) with logarithmic speed scaling for macro/micro views.
- [ ] **S1.6:** Keyboard controls — pause/resume, simulation speed multiplier (1×, 10×, 100×, 1000×).
- [ ] **S1.7:** Entity selection — click on body to focus camera + display info.
- [ ] **S1.8:** Frame timing display — FPS, physics tick rate, total entity count.

---

## Phase S2: Rendering Pipeline

- [ ] **S2.1:** Instanced draw calls — single draw for thousands of sphere primitives.
- [ ] **S2.2:** Vertex + fragment shaders for Phong-lit sphere rendering.
- [ ] **S2.3:** Per-body color from Temperature via Wien's law (realistic star color mapping).
- [ ] **S2.4:** Size scaling based on Mass (logarithmic mapping for visual clarity).
- [ ] **S2.5:** Orbit trail renderer — line-strip of last N positions per tracked entity.
- [ ] **S2.6:** Velocity vector arrows (toggleable overlay).
- [ ] **S2.7:** Force vector arrows (toggleable overlay).
- [ ] **S2.8:** Grid / sector boundary wireframe overlay.
- [ ] **S2.9:** SPH fluid particle rendering — color-mapped by density or pressure.
- [ ] **S2.10:** Procedural skybox / starfield background.
- [ ] **S2.11:** Bloom post-processing for luminous bodies (stars, supernovae).
- [ ] **S2.12:** Gravitational lensing post-process shader — Schwarzschild ray-marching distortion.

---

## Phase S3: Debug UI & Developer Tools

- [ ] **S3.1:** On-screen HUD — selected entity properties (mass, velocity, position, temperature, type).
- [ ] **S3.2:** Real-time energy monitor graph — kinetic, potential, and total energy curves.
- [ ] **S3.3:** Entity spawner — click-to-place bodies with configurable mass, velocity, and type.
- [ ] **S3.4:** Console/terminal overlay — runtime commands: change `G`, `ε`, `dt`, integrator, spawn presets.
- [ ] **S3.5:** World state serialization — save/load full simulation state to binary or JSON.
- [ ] **S3.6:** Time control UI — speed slider, step-by-step mode, reverse playback of saved states.
- [ ] **S3.7:** Performance profiler overlay — per-system timing breakdown (gravity, integration, SPH, collisions).
- [ ] **S3.8:** Entity inspector panel — scrollable list of all entities, filterable by `BodyType`.

---

## Phase S4: Creative Scenarios & Presets

> *Each scenario is a self-contained initial state that demonstrates a specific physics phenomenon.
> The system is designed to be extensible — adding a new scenario requires only defining initial
> entity spawns and configuration, with zero engine modifications.*

- [ ] **S4.1:** 🌍 **Solar System** — Sun + 8 planets + major moons, real masses/distances/velocities from JPL ephemerides.
- [ ] **S4.2:** ⭐ **Binary Star System** — two stars in mutual orbit with a circumbinary planet.
- [ ] **S4.3:** ☄️ **Asteroid Belt** — thousands of particles in orbital band with Jupiter gravitational perturbations.
- [ ] **S4.4:** 🌀 **Galaxy Formation** — NFW dark matter halo + 100K star particles with spiral arm dynamics.
- [ ] **S4.5:** 🔥 **Star Formation Nebula** — SPH gas cloud collapsing under self-gravity → protostar ignition.
- [ ] **S4.6:** 💧 **Fluid Playground** — dam break, droplet collision, vortex rings (SPH showcase).
- [ ] **S4.7:** 💥 **Collision Lab** — configurable body collisions: elastic, inelastic, explosive fragmentation.
- [ ] **S4.8:** 🕳️ **Black Hole Accretion** — particles spiraling into Schwarzschild geometry with gravitational lensing.
- [ ] **S4.9:** ⚡ **Electromagnetic Sandbox** — charged particles in magnetic fields, plasma confinement, Larmor orbits.
- [ ] **S4.10:** 🌙 **Tidal Disruption** — moon crossing Roche limit → tidal breakup into ring system.
- [ ] **S4.11:** 💫 **Supernova** — stellar core collapse, shockwave propagation through SPH gas envelope.
- [ ] **S4.12:** 🎲 **Three-Body Chaos** — interactive 3-body problem with real-time Lyapunov exponent divergence display.

---

*End of Backlog. Total: 13 engine phases (~100 tasks) + 4 sandbox phases (~40 tasks).*
*Extensible by design — new scenarios (S4.N+1) and physics modules (Phase N+1) can be appended without restructuring.*
