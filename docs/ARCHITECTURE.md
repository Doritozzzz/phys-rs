# Universal Physics Engine: Architecture Reference

This document outlines the Data-Oriented Design (DOD) architecture for `phys-rs`, enabling an "Infinity Sandbox" scale without floating-point precision loss.

## 1. The Hierarchical Floating Origin System

To prevent floating-point drift at astronomical distances, the engine employs a dual-coordinate hierarchical system.

### Coordinate Types

- **Sector (`i128`)**: Defines a rigid, absolute grid cell in the universe. The scale of a sector is configurable (defaulting to 1 Light Year, $9.46 \times 10^{15}$ meters) via the `UniverseConfig` resource.
- **Local (`DVec3`)**: An `f64` 3D vector representing the precise position of an entity *relative to the center of its current Sector*.

```rust
use bevy_ecs::prelude::*;
use glam::DVec3;

// Configurable sector grid size
#[derive(Resource)]
pub struct UniverseConfig {
    pub sector_size_meters: f64, 
}

#[derive(Component)]
pub struct Sector {
    pub x: i128,
    pub y: i128,
    pub z: i128,
}

#[derive(Component)]
pub struct LocalPosition(pub DVec3);
```

### The OriginShift System
An observer (camera or focus entity) tracks the origin. When the observer's `LocalPosition` exceeds a configured threshold (e.g., `sector_size_meters / 2.0`), the system:
1. Calculates the offset to the new sector.
2. Updates the observer's `Sector`.
3. Subtracts the offset from the `LocalPosition` of *all active entities* within physics range.
4. Updates the `Sector` index for any entities that crossed the boundary.

## 2. GPGPU State Persistence (Compute Sync)

Physics integration occurs on the GPU using `wgpu` compute shaders to handle millions of particles (SPH) and N-Body mechanics.

### Memory Layout & Data Oriented Design
We use the `encase` crate to guarantee that Rust's ECS structs perfectly align with WGSL `StorageBuffers`.

- **Source of Truth**: The GPU VRAM holds the definitive physics state (Position, Velocity, Mass).
- **ECS Role**: The `bevy_ecs` instance on the CPU acts as the "Director". It dispatches commands, handles broad-phase sector logic, and manages input/AI.
- **Synchronization**: We only read back data from the GPU to the CPU when absolutely necessary (e.g., origin shifts across sectors, or user queries). For rendering, `wgpu` directly uses the physics buffers, maintaining zero-copy rendering.

```rust
// Example Encase alignment for WGSL
use encase::ShaderType;
use glam::DVec3;

#[derive(ShaderType)]
pub struct GpuPhysicsData {
    pub local_pos: DVec3,
    pub velocity: DVec3,
    pub mass: f64,
    // Padding managed by encase
}
```

## 3. Spatial Partitioning

- **Broad-Phase**: Grid-based hashing based on `Sector`.
- **Narrow-Phase / Local Space**: 
    - `kiddo` (K-D Trees) for hyper-fast SPH nearest-neighbor searches.
    - `parry3d-f64` (BVH) for complex rigid-body manifold generation.
