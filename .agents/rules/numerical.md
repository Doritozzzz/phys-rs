---
trigger: always_on
glob:
description: Mandatory numerical stability and floating-point safety rules for all physics code in phys-rs.
---

# Rule: Numerical Stability & Floating-Point Safety

This rule is MANDATORY and applies to every line of code written in this project. The phys-rs engine simulates physics at astronomical scales where a single NaN or Infinity will cascade and destroy the entire simulation state. There are NO exceptions to these protocols, even if the user explicitly asks to skip them.

## 1. Precision Standard

- **ALL physical quantities** (position, velocity, acceleration, force, mass, energy, time) MUST use `f64` (double precision).
- **ALL vector types** for physics MUST use `glam::DVec3`, never `Vec3` (which is `f32`).
- **Physical constants** MUST be declared as `f64` literals with the `_f64` suffix. Example: `6.67430e-11_f64`, never `6.67430e-11` without the suffix.
- **NEVER use `f32`** for any physics calculation, component, or constant. The only acceptable use of `f32` is in the rendering pipeline (`wgpu` vertex buffers, color data) where GPU hardware requires it.
- If the user asks for `f32` physics, REFUSE and explain that `docs/ARCHITECTURE.md` Section 2 mandates `f64` precision. This is a hard architectural constraint, not a suggestion.

## 2. Singularity Prevention (Softening)

Any force law with an inverse-distance term ($1/r^n$) will produce `Inf` when $r = 0$. This is the most common cause of simulation explosions.

- **Gravitational force** MUST use the softened form: $F = G \frac{m_1 m_2}{r^2 + \epsilon^2}$
- **Coulomb force** MUST use the softened form: $F = k_e \frac{q_1 q_2}{r^2 + \epsilon^2}$
- The softening parameter $\epsilon$ MUST be a configurable constant, not hardcoded inline. Define it as a `const` or as a field in a configuration `Resource`.
- A reasonable default value for gravitational softening is `1e-4` (meters), but this depends on the simulation scale and should be documented.
- **NEVER write a bare division by `r^2` or `r^3`** in any force calculation. Always add the softening term.

## 3. Finite Result Validation

After computing any accumulated force, integrated velocity, or updated position:

- **ALWAYS assert finiteness**: Use `debug_assert!(value.is_finite())` for per-component scalar checks, or `debug_assert!(vec.is_finite())` for `DVec3` values.
- Place these assertions AFTER the computation but BEFORE writing the result back to an ECS component.
- Use `debug_assert!` (not `assert!`) so that checks are compiled out in `--release` builds for performance, but are active during development.
- If you are writing a system that accumulates forces from multiple sources, validate the FINAL accumulated result, not each individual contribution (to avoid performance overhead).

## 4. Delta Time (dt) Discipline

- The simulation MUST use a **fixed timestep** (`dt` is constant across all ticks). This is required by the `docs/ARCHITECTURE.md` Section 3 determinism constraint.
- Variable `dt` is FORBIDDEN in physics systems. If you receive a variable frame delta, it must be consumed by an accumulator pattern that calls the physics step with a fixed `dt`.
- If implementing the Velocity Verlet integrator, the fixed `dt` is what preserves its symplectic (energy-conserving) property. A variable `dt` would break energy conservation.
- When defining `dt`, always clamp it: `dt.min(MAX_DT).max(MIN_DT)` as a safety net, even if it's supposed to be constant.

## 5. Overflow & Underflow Protection

- When multiplying very large numbers (masses of stars: ~$10^{30}$ kg) by very small numbers (G: ~$10^{-11}$), be aware of intermediate overflow even in `f64`.
- **Prefer multiplication order that keeps intermediate values moderate.** Example: compute `G * m1` first (large × small = moderate), then multiply by `m2`, then divide. Don't compute `m1 * m2` first (large × large = very large).
- For energy calculations, validate that kinetic energy ($\frac{1}{2}mv^2$) and potential energy ($-G\frac{m_1 m_2}{r}$) remain within physically reasonable bounds. Log a warning if total energy drifts more than a configurable threshold from the initial value.

## 6. Integration Method Safety

- **Semi-Implicit Euler**: Acceptable for prototyping. Update velocity BEFORE position (symplectic order).
- **Velocity Verlet**: Preferred for N-body. Requires storing the previous acceleration. Never skip the half-step acceleration recalculation.
- **RK4**: Use only when orbital precision demands it. Be aware of 4× the force evaluations per step.
- **NEVER use standard (explicit) Euler**: $r_{n+1} = r_n + v_n \cdot dt$, $v_{n+1} = v_n + a_n \cdot dt$ in that order. This is energy-divergent and will cause orbits to spiral outward.
