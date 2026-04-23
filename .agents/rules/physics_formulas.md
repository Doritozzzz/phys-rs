---
trigger: always_on
glob: src/physics/**
description: Enforces that all physics implementations derive from the canonical equations in docs/PHYSICS_MASTER_INDEX.md. Prevents formula errors, incorrect signs, and missing terms.
---

# Rule: Physics Formula Compliance

Every physics system in this engine implements a real physical law. The canonical source of truth for all equations is `docs/PHYSICS_MASTER_INDEX.md`. This rule ensures that implementations match the math exactly.

## 1. Before Writing Any Physics Code

1. **Open `docs/PHYSICS_MASTER_INDEX.md`** and locate the exact equation you are implementing.
2. **Copy the equation into the doc comment** of the function/system, citing its section number.
3. If the equation is NOT in the index, **update the index first** before writing code. No physics code may exist without a corresponding entry in the master index.

## 2. Gravitational Force (§IV.1)

The canonical vector form is:

$$\vec{F}_{12} = -G \frac{m_1 m_2}{|\vec{r}_{12}|^3} \vec{r}_{12}$$

Implementation requirements:
- The sign convention: `r_vec = pos_2 - pos_1` means the force on body 1 points TOWARD body 2 (attractive). Apply Newton's third law: `force_on_1 = +force_vec`, `force_on_2 = -force_vec`.
- Use the softened denominator: $r^2 + \epsilon^2$ (see numerical stability rule).
- The divisor is $r^3$ (not $r^2$) because we multiply by the full $\vec{r}$ vector instead of the unit vector $\hat{r}$. This avoids a separate normalization + magnitude computation.

## 3. Integration Methods (§I)

### Semi-Implicit Euler (§I.1)
```
v_new = v + a * dt      // velocity FIRST
r_new = r + v_new * dt   // then position using NEW velocity
```
**CRITICAL**: The order matters. Updating position with the OLD velocity is standard Euler (non-symplectic, energy-divergent). Always use the NEW velocity.

### Velocity Verlet (§I.2)
```
r_new = r + v * dt + 0.5 * a * dt²
a_new = compute_forces(r_new) / m     // recompute acceleration at new position
v_new = v + 0.5 * (a + a_new) * dt
```
**CRITICAL**: You MUST store the previous acceleration and recompute forces at the new position. Skipping the mid-step acceleration recalculation degrades this to standard Euler.

### RK4 (§I.3)
All four stages ($k_1$ through $k_4$) must be computed. Each stage evaluates forces at an intermediate state. The final combination weights are $\frac{1}{6}(k_1 + 2k_2 + 2k_3 + k_4)$. Do not approximate by skipping $k_3$ or $k_4$.

## 4. Collision Impulse (§III.1)

The impulse scalar $j$ includes rotational inertia terms. Do NOT simplify to the particle-only form ($j = \frac{-(1+e)v_{rel} \cdot n}{1/m_A + 1/m_B}$) unless the bodies have infinite moment of inertia (i.e., are point particles with no rotation).

## 5. SPH (§V)

- Density estimation (§V.1) MUST include self-contribution: particle $i$ contributes to its own density.
- The pressure gradient (§V.3) uses the symmetric form $\frac{P_i}{\rho_i^2} + \frac{P_j}{\rho_j^2}$ to ensure momentum conservation. Do NOT use the asymmetric form.
- Always include Monaghan artificial viscosity (§V.3) when particles approach each other to prevent interpenetration.

## 6. Unit System

All values are in SI units. No unit conversions should happen inside physics systems.

| Quantity | Unit | Typical Range |
|---|---|---|
| Distance | meters (m) | $10^{-3}$ to $10^{20}$ |
| Mass | kilograms (kg) | $10^{-3}$ to $10^{30}$ |
| Time | seconds (s) | $10^{-6}$ to $10^{10}$ |
| Force | newtons (N) | derived |
| Energy | joules (J) | derived |
| Temperature | kelvin (K) | $3$ to $10^{9}$ |

If a user provides values in non-SI units (e.g., solar masses, AU, light-years), convert them to SI at the INPUT boundary, never inside a physics system.

## 7. Conservation Law Verification

After implementing any force or integrator, verify conservation:
- **Gravity + Verlet**: Total mechanical energy ($E_k + E_p$) should remain constant (within floating-point drift) over $10^4+$ steps in a two-body orbit.
- **Collisions**: Total linear momentum before = total linear momentum after (within $\epsilon$).
- **SPH**: Total mass is conserved (no particles created/destroyed).

If conservation is violated beyond acceptable thresholds, the implementation has a bug. Do not merge it.
