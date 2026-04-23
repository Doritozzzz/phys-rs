
# phys-rs — Universal Physics Engine

A high-performance, deterministic physics simulation engine written in Rust. Designed to simulate physical phenomena from particle collisions to galaxy formation — all in double precision, all from first principles.

## Architecture

**Engine-first design:** The physics engine is a pure simulation library with zero visual dependencies. It runs headlessly, is fully testeable with `cargo test`, and produces deterministic results across platforms.

**Sandbox layer:** An interactive visual playground built on top of the engine using `wgpu` + `winit`. Provides real-time 3D rendering, camera controls, debug tools, and pre-built scenarios.

### Technical Stack

| Layer | Technology | Purpose |
|---|---|---|
| **ECS** | `bevy_ecs` | Entity Component System for data-oriented physics |
| **Math** | `glam` (DVec3/DQuat) + `nalgebra` (Matrix3) | SIMD-optimized f64 linear algebra |
| **Collision** | `parry3d-f64` + `kiddo` | Narrow-phase manifolds + K-D tree broadphase |
| **GPU** | `wgpu` + `encase` | Compute shaders + buffer marshalling |
| **Window** | `winit` | Cross-platform windowing and input |

## Engine Capabilities

### Dynamics & Forces
- N-body gravitation (brute-force + Barnes-Hut O(N log N))
- Rigid body dynamics with full inertia tensors
- Collision detection and impulse resolution
- Electrostatics (Coulomb) and magnetism (Lorentz force)

### Fluid Simulation
- Smoothed Particle Hydrodynamics (SPH) with multiple kernels
- Monaghan artificial viscosity for shock stability
- Magnetohydrodynamics (MHD) for stellar plasmas

### Thermodynamics
- Heat conduction (Fourier's law)
- Blackbody radiation (Stefan-Boltzmann)
- Ideal gas and Tait equations of state

### Advanced Physics
- Relativistic momentum corrections
- Schwarzschild geodesics for photon paths
- NFW dark matter halo profiles
- Nuclear astrophysics: fusion rates, degeneracy pressure, stellar lifecycle

### Numerical Methods
- Semi-Implicit Euler, Velocity Verlet, RK4 integrators
- Fixed-timestep accumulator for determinism
- Softened force laws to prevent singularities
- IEEE 754 f64 precision throughout

## Sandbox Scenarios

Pre-built interactive demonstrations:
- 🌍 Solar System — real ephemeris data
- ⭐ Binary Star + circumbinary planet
- 🌀 Galaxy formation with dark matter halos
- 🔥 Star formation from SPH gas cloud collapse
- 💧 Fluid playground (dam break, vortex rings)
- 🕳️ Black hole accretion with gravitational lensing
- 💫 Supernova shockwave propagation
- 🎲 Three-body chaos with Lyapunov divergence
- *...and more*

## Development

### Prerequisites

- Rust Toolchain (Stable, edition 2024)
- Vulkan / DirectX 12 / Metal compatible GPU

### Build & Run

```bash
# Debug build
cargo build

# Run with all features
cargo run --release

# Run with nuclear astrophysics
cargo run --release --features nuclear

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## Documentation

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — Module structure, dependencies, design constraints
- [`docs/PHYSICS_MASTER_INDEX.md`](docs/PHYSICS_MASTER_INDEX.md) — Canonical equations for all physics systems
- [`docs/BACKLOG.md`](docs/BACKLOG.md) — Complete engineering task backlog
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — High-level phase objectives and deliverables
