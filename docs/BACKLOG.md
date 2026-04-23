# Engineering Backlog: Universal Physics Engine

## Phase 0: System Bootstrap & Architecture
- [ ] **Task 0.1:** Initialize workspace (`core`, `physics`, `gpu`, `math`).
- [ ] **Task 0.2:** Setup `Cargo.toml` with `kiddo`, `parry3d-f64`, `encase`, `nalgebra`, `bevy_floating_origin`.
- [ ] **Task 0.3:** Setup `wgpu` context and compute shader scaffolding.

## Phase 1: Infinite Scale Infrastructure
- [ ] **Task 1.1:** Implement the `UniverseConfig` resource for configurable Sector sizes (e.g., 1 LY to 100km testing sizes).
- [ ] **Task 1.2:** Implement `Sector` (i128) and `LocalPosition` (DVec3) components.
- [ ] **Task 1.3:** Create the `OriginShift` system to handle floating origin translation logic.
- [ ] **Task 1.4:** Build GPU data mapping pipeline using `encase` for `StorageBuffer` synchronization.

## Phase 2: The f64 GPU Integrator
- [ ] **Task 2.1:** Implement WGSL Compute Shaders for Semi-Implicit Euler.
- [ ] **Task 2.2:** Implement WGSL Compute Shaders for Velocity Verlet (N-Body conserved).
- [ ] **Task 2.3:** Implement WGSL Compute Shaders for RK4 (High-precision orbital).
- [ ] **Task 2.4:** Implement Sub-stepping logic for collision stability at high velocities.
- [ ] **Task 2.5:** Unit Test: Earth-Moon orbit tracking energy conservation over 10M ticks.

## Phase 3: Spatial Partitioning & Collisions
- [ ] **Task 3.1:** Integrate `kiddo` for CPU-side / GPU-accelerated K-D tree neighbor lookups.
- [ ] **Task 3.2:** Integrate `parry3d-f64` for rigid-body collision shapes and manifold generation.
- [ ] **Task 3.3:** Implement Dual-Quaternion (`nalgebra`) rotational integration to avoid gimbal lock and drift.

## Phase 4: Advanced Fluids & SPH
- [ ] **Task 4.1:** Implement standard SPH density and pressure gradients.
- [ ] **Task 4.2:** Implement Monaghan Artificial Viscosity for shock stability.
- [ ] **Task 4.3:** Implement Magnetohydrodynamics (MHD) Induction Equation for plasma.

## Phase 5: Cosmology & Relativistic Physics
- [ ] **Task 5.1:** Integrate Navarro-Frenk-White (NFW) Dark Matter Profiles for galactic clusters.
- [ ] **Task 5.2:** Implement Relativistic Doppler shift and Lorentz transforms.
- [ ] **Task 5.3:** Implement Schwarzschild geodesics for Ray-marched rendering and photon paths.
- [ ] **Task 5.4:** Implement Lense-Thirring effect (Frame-dragging) near massive rotating bodies.
