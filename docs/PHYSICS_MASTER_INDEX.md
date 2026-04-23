# PHYSICS MASTER INDEX

This document contains the absolute mathematical foundation for the Universal Physics Engine. All systems implemented in the codebase must derive their logic from the formulas listed below.

All quantities assume the International System of Units (SI): Meters (m), Kilograms (kg), Seconds (s), Newtons (N), Joules (J), Kelvin (K), Coulombs (C).

---

## I. NUMERICAL INTEGRATION (The Core Engine)

These formulas define how time progresses and how derivatives translate into state changes over $\Delta t$.

### 1. Semi-Implicit Euler (Symplectic)

Used for basic, stable kinematics.

$$
\vec{v}_{t+\Delta t} = \vec{v}_t + \vec{a}_t \Delta t
$$

$$
\vec{r}_{t+\Delta t} = \vec{r}_t + \vec{v}_{t+\Delta t} \Delta t
$$

### 2. Velocity Verlet

Standard for N-Body simulations; conserves energy over long periods.

$$
\vec{r}_{t+\Delta t} = \vec{r}_t + \vec{v}_t \Delta t + \frac{1}{2} \vec{a}_t \Delta t^2
$$

$$
\vec{a}_{t+\Delta t} = f(\vec{r}_{t+\Delta t})
$$

$$
\vec{v}_{t+\Delta t} = \vec{v}_t + \frac{\vec{a}_t + \vec{a}_{t+\Delta t}}{2} \Delta t
$$

### 3. Runge-Kutta 4th Order (RK4)

Used for high-precision orbital mechanics where error minimization is critical.
For a state vector $\vec{y}$ and differential equation $\frac{d\vec{y}}{dt} = f(t, \vec{y})$:

$$
\vec{k}_1 = f(t_n, \vec{y}_n)
$$

$$
\vec{k}_2 = f(t_n + \frac{\Delta t}{2}, \vec{y}_n + \frac{\Delta t}{2} \vec{k}_1)
$$

$$
\vec{k}_3 = f(t_n + \frac{\Delta t}{2}, \vec{y}_n + \frac{\Delta t}{2} \vec{k}_2)
$$

$$
\vec{k}_4 = f(t_n + \Delta t, \vec{y}_n + \Delta t \vec{k}_3)
$$

$$
\vec{y}_{n+1} = \vec{y}_n + \frac{\Delta t}{6} (\vec{k}_1 + 2\vec{k}_2 + 2\vec{k}_3 + \vec{k}_4)
$$

---

## II. CLASSICAL MECHANICS & RIGID BODY DYNAMICS

### 1. Linear Dynamics

* **Newton's Second Law:** $\sum \vec{F} = m\vec{a}$
* **Linear Momentum:** $\vec{p} = m\vec{v}$
* **Impulse:** $\vec{J} = \int \vec{F} dt = \Delta \vec{p}$

### 2. Rotational Dynamics

* **Torque:** $\vec{\tau} = \vec{r} \times \vec{F}$
* **Angular Momentum:** $\vec{L} = \vec{r} \times \vec{p} = I\vec{\omega}$
* **Angular Acceleration:** $\vec{\alpha} = I^{-1} (\vec{\tau} - \vec{\omega} \times (I\vec{\omega}))$
* **Moment of Inertia (Solid Sphere):** $I = \frac{2}{5}mr^2$

### 3. Quaternion Rotation Update

To avoid gimbal lock, angular velocity $\vec{\omega}$ updates the orientation quaternion $q$:

$$
\frac{dq}{dt} = \frac{1}{2} \omega q
$$

$$
q_{t+\Delta t} = q_t + \frac{1}{2} ( [0, \vec{\omega}] \otimes q_t ) \Delta t
$$

### 4. Damping & Friction

* **Kinetic Friction:** $F_k = \mu_k F_N$
* **Aerodynamic Drag:** $\vec{F}_d = -\frac{1}{2} \rho |\vec{v}|^2 C_d A \hat{v}$
* **Stokes' Law (Viscous Drag):** $\vec{F}_d = -6\pi \eta r \vec{v}$

---

## III. COLLISION RESOLUTION (Manifold Generation)

### 1. Elastic Impulse (Two Bodies)

For bodies A and B with coefficient of restitution $e$, normal $\vec{n}$, and relative velocity $\vec{v}_{rel} = \vec{v}_A - \vec{v}_B$:

$$
j = \frac{-(1 + e)\vec{v}_{rel} \cdot \vec{n}}{\frac{1}{m_A} + \frac{1}{m_B} + \left( I_A^{-1}(\vec{r}_A \times \vec{n}) \times \vec{r}_A + I_B^{-1}(\vec{r}_B \times \vec{n}) \times \vec{r}_B \right) \cdot \vec{n}}
$$

* **Velocity Update:** 

  $$
  \vec{v}_A' = \vec{v}_A + \frac{j}{m_A}\vec{n}
  $$

  $$
  \vec{v}_B' = \vec{v}_B - \frac{j}{m_B}\vec{n}
  $$

---

## IV. CELESTIAL MECHANICS & GRAVITATION

### 1. Universal Gravitation

* **Newton's Law of Universal Gravitation:** 
  $$
  \vec{F}_{12} = -G \frac{m_1 m_2}{|\vec{r}_{12}|^3} \vec{r}_{12}
  $$
* **Gravitational Potential Energy:** $U_g = -G\frac{m_1 m_2}{r}$

### 2. Orbital Parameters

* **Vis-Viva Equation (Velocity in orbit):**
  $$
  v = \sqrt{GM \left( \frac{2}{r} - \frac{1}{a} \right)}
  $$
* **Escape Velocity:** $v_e = \sqrt{\frac{2GM}{r}}$
* **Orbital Period:** $T = 2\pi \sqrt{\frac{a^3}{GM}}$

### 3. Astrophysical Limits

* **Roche Limit (Rigid satellite):** $d = 2.44 R_M \left( \frac{\rho_M}{\rho_m} \right)^{1/3}$
* **Schwarzschild Radius (Black Hole Event Horizon):** $r_s = \frac{2GM}{c^2}$

---

## V. FLUID DYNAMICS (Smoothed Particle Hydrodynamics - SPH)

Formulas for simulating gases, nebulae, and liquid bodies.

### 1. Density & Interpolation

Given a smoothing kernel $W(\vec{r}, h)$ where $h$ is the smoothing radius:

* **Density at particle $i$:** 
  $$
  \rho_i = \sum_j m_j W(\vec{r}_i - \vec{r}_j, h)
  $$

### 2. Equation of State (Pressure Calculation)

* **Ideal Gas (Stars/Nebulae):** $P_i = k(\rho_i - \rho_0)$
* **Tait Equation (Liquids):** $P_i = B \left( \left(\frac{\rho_i}{\rho_0}\right)^\gamma - 1 \right)$

### 3. SPH Forces & Stability

* **Pressure Force Gradient:**
  $$
  \vec{F}_i^{press} = -m_i \sum_j m_j \left( \frac{P_i}{\rho_i^2} + \frac{P_j}{\rho_j^2} \right) \nabla W(\vec{r}_i - \vec{r}_j, h)
  $$
* **Viscosity Force:**
  $$
  \vec{F}_i^{visc} = \mu m_i \sum_j m_j \left( \frac{\vec{v}_j - \vec{v}_i}{\rho_i \rho_j} \right) \nabla^2 W(\vec{r}_i - \vec{r}_j, h)
  $$

* **Monaghan Artificial Viscosity (Shock Stability):**
  Prevents particle interpenetration at high-speed impacts.
  $$ \Pi_{ij} = \frac{-\alpha \bar{c}_{ij} \mu_{ij} + \beta \mu_{ij}^2}{\bar{\rho}_{ij}} $$
  *(Added to the pressure terms when particles approach each other).*

### 4. Magnetohydrodynamics (MHD)
Coupling fluid velocity with magnetic fields for stellar plasmas.
* **Induction Equation:**
  $$ \frac{\partial \vec{B}}{\partial t} = \nabla \times (\vec{v} \times \vec{B}) + \eta \nabla^2 \vec{B} $$

---

## VI. THERMODYNAMICS & HEAT TRANSFER

### 1. State and Energy

* **Ideal Gas Law:** $PV = nRT$ or $P = \rho R_{specific} T$
* **Internal Energy:** $U = \frac{3}{2}N k_B T$ (Monatomic gas)

### 2. Heat Transfer

* **Conduction (Fourier's Law):** $\vec{q} = -k \nabla T$
* **Thermal Radiation (Stefan-Boltzmann):** Power emitted per unit area: $j^* = \sigma \epsilon T^4$
* **Wien's Displacement Law (Color of a star):** $\lambda_{max} = \frac{b}{T}$

---

## VII. ELECTROMAGNETISM & OPTICS

### 1. Maxwell's Equations (Differential Form)

* **Gauss's Law:** $\nabla \cdot \vec{E} = \frac{\rho}{\varepsilon_0}$
* **Gauss's Law for Magnetism:** $\nabla \cdot \vec{B} = 0$
* **Faraday's Law:** $\nabla \times \vec{E} = -\frac{\partial \vec{B}}{\partial t}$
* **Ampère-Maxwell Law:** $\nabla \times \vec{B} = \mu_0 \vec{J} + \mu_0 \varepsilon_0 \frac{\partial \vec{E}}{\partial t}$

### 2. Particle Interactions

* **Lorentz Force:** $\vec{F} = q(\vec{E} + \vec{v} \times \vec{B})$
* **Coulomb's Law:** $\vec{F} = \frac{1}{4\pi\varepsilon_0} \frac{q_1 q_2}{r^2} \hat{r}$

### 3. Light Propagation

* **Cosmological Redshift:** $1 + z = \frac{\lambda_{obs}}{\lambda_{emit}} = \sqrt{\frac{1 + \beta}{1 - \beta}}$
* **Photon Energy:** $E = hf = \frac{hc}{\lambda}$

---

## VIII. RELATIVISTIC KINEMATICS

For objects approaching $c$ ($3 \times 10^8$ m/s).

* **Lorentz Factor:** $\gamma = \frac{1}{\sqrt{1 - \frac{v^2}{c^2}}}$
* **Time Dilation:** $\Delta t' = \gamma \Delta t$
* **Length Contraction:** $L = \frac{L_0}{\gamma}$
* **Relativistic Momentum:** $\vec{p} = \gamma m_0 \vec{v}$
* **Energy-Momentum Relation:** $E^2 = (pc)^2 + (m_0 c^2)^2$
* **Relativistic Doppler Shift:**
  $$ f_{obs} = f_{emit} \sqrt{\frac{1 - v/c}{1 + v/c}} $$

---

## IX. NUCLEAR ASTROPHYSICS (Stellar Cores)

### 1. Mass Defect and Fusion Yield

* **Mass-Energy Equivalence:** $E = \Delta m c^2$
* **Reaction Rate (Gamow Peak approximation):** $r_{AB} \propto n_A n_B T^{-2/3} \exp \left[ -3 \left( \frac{\pi^2 z_A^2 z_B^2 e^4 m_r}{2 \hbar^2 k_B T} \right)^{1/3} \right]$

### 2. Quantum Limits

* **Heisenberg Uncertainty Principle:** $\Delta x \Delta p \geq \frac{\hbar}{2}$
* **Degeneracy Pressure (Fermi Gas limit):** $P \propto \rho^{5/3}$ (Prevents collapse in non-fusing cores).

---

## X. ADVANCED COSMOLOGY & GENERAL RELATIVITY

For galactic scales and extreme gravitational environments.

### 1. Galactic Dynamics
* **Navarro-Frenk-White (NFW) Dark Matter Profile:**
  Defines the density of dark matter halos to prevent outer stars from flying off.
  $$ \rho(r) = \frac{\rho_0}{\frac{r}{R_s} \left(1 + \frac{r}{R_s}\right)^2} $$

### 2. Spacetime Curvature
* **Schwarzschild Geodesics:**
  Equation of motion for particles (and light) in curved spacetime around a non-rotating black hole.
  $$ \frac{d^2 x^\mu}{d\tau^2} + \Gamma^\mu_{\alpha\beta} \frac{dx^\alpha}{d\tau} \frac{dx^\beta}{d\tau} = 0 $$
* **Lense-Thirring Effect (Frame-Dragging):**
  Induced precession of inertial frames around a massive, rotating body.
  $$ \vec{\Omega}_{LT} = \frac{G}{c^2 r^3} \left[ \frac{3(\vec{J} \cdot \vec{r})\vec{r}}{r^2} - \vec{J} \right] $$

---

*End of Reference.*
