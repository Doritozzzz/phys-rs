---
description: Workflow to compile and validate WGSL compute shaders for the wgpu GPU pipeline. Ensures shader code is correct before runtime.
---

# Workflow: Build & Validate GPU Shaders

Use this workflow when creating or modifying WGSL compute shaders for the `wgpu` pipeline. Shader compilation errors only surface at runtime, so this workflow catches them early.

## Step 1: Write the Shader

1. Create or modify a `.wgsl` file in the appropriate location (e.g., `src/render/shaders/` or `src/physics/shaders/`).
2. Ensure the WGSL struct layout matches the Rust side exactly:
   - Use `encase` annotations on the Rust struct to guarantee layout compatibility.
   - WGSL `f32` maps to Rust `f32` (GPU-side only). Physics data transferred as `f64` must be split or converted at the boundary.
   - Respect alignment rules: `vec3<f32>` in WGSL has 16-byte alignment (same as `vec4`).

## Step 2: Validate with naga-cli

// turbo
```bash
cargo install naga-cli
```

// turbo
```bash
naga --validate src/render/shaders/my_shader.wgsl
```

If `naga` reports errors, fix them before proceeding. Common issues:
- **Mismatched binding indices**: Ensure `@group(0) @binding(N)` matches the Rust `BindGroupLayout`.
- **Type errors**: WGSL is strictly typed. No implicit conversions between `f32`, `i32`, `u32`.
- **Missing return**: Compute shader entry points must be `@compute @workgroup_size(X, Y, Z)`.

## Step 3: Verify encase Compatibility

// turbo
```bash
cargo test --release -- shader_layout --nocapture
```

Write a test that creates the `encase::StorageBuffer`, writes a known value, and reads it back to verify byte layout matches expectations.

## Step 4: Runtime Smoke Test

1. Create a minimal test that:
   - Initializes `wgpu` with a compute pipeline using your shader.
   - Uploads a small buffer with known input values.
   - Dispatches the compute shader.
   - Downloads and verifies the output buffer.
2. Compare GPU output against a CPU reference implementation to catch precision or logic errors.

// turbo
```bash
cargo test --release -- gpu_smoke --nocapture
```

## Step 5: Performance Baseline

For compute-heavy shaders (N-body, SPH kernels):

// turbo
```bash
cargo bench -- shader_perf
```

Record the baseline throughput (particles/second or GFLOPS) so that future modifications can be compared against it.
