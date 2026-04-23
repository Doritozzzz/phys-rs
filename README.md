
# Universal Physics Engine

A high-performance, deterministic physics simulation framework implemented in Rust. This project focuses on data-oriented design and hardware-accelerated computing to simulate complex celestial mechanics and fluid dynamics.

## Core Architecture

The engine is built upon the **Entity Component System (ECS)** pattern, ensuring high cache locality and efficient parallel processing of large-scale simulations.

### Technical Stack

- **Engine Core:** `bevy_ecs` for decoupled logic and data management.
- **Linear Algebra:** `glam` for SIMD-optimized vector and matrix operations.
- **Numerical Integration:** Support for Higher-order Runge-Kutta (RK4) and Velocity Verlet schemes.
- **Graphics API:** `wgpu` (WebGPU) for cross-platform, hardware-accelerated rendering via instanced draws.
- **Windowing:** `winit` for native event handling.

## System Modules

### 1. Physics & Dynamics

Implementation of N-body gravitational interactions and collision manifolds. The system is designed to handle:

- Universal Gravitation with customizable G constants.
- Linear and angular momentum conservation.
- Atmospheric drag models and fluid-particle interactions (SPH).

### 2. Spatial Partitioning

To maintain $O(n \log n)$ complexity in large environments, the engine utilizes:

- **Octrees:** For efficient spatial queries and gravitational approximations.
- **Barnes-Hut Algorithm:** For accelerated N-body calculations.

### 3. Rendering Pipeline

- Procedural geometry generation.
- Modern programmable shading for density and temperature visualization.
- High-buffer throughput for massive particle counts.

## Development

### Prerequisites

- Rust Toolchain (Stable)
- Vulkan/DirectX12/Metal compatible hardware

### Build Instructions

```bash
# Debug build
cargo build

# Optimized release build
cargo build --release

# Execution
cargo run --release
```
