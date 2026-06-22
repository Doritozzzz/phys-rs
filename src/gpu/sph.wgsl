struct SphParams {
    num_particles: u32,
    mass: f32,
    h: f32,
    dt: f32,
    rest_density: f32,
    speed_of_sound: f32,
    gamma: f32,
    alpha_visc: f32,
    beta_visc: f32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read_write> positions: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> velocities: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> densities: array<f32>;
@group(0) @binding(3) var<storage, read_write> pressures: array<f32>;
@group(0) @binding(4) var<storage, read_write> forces: array<vec4<f32>>;
@group(0) @binding(5) var<uniform> params: SphParams;

const PI: f32 = 3.14159265359;
const EPS2: f32 = 1e-12;

fn cubic_spline(r: f32, h: f32) -> f32 {
    let q = r / h;
    if (q >= 2.0) { return 0.0; }
    let sigma = 1.0 / (PI * h * h * h);
    if (q < 1.0) {
        return sigma * (1.0 - 1.5 * q * q + 0.75 * q * q * q);
    } else {
        let diff = 2.0 - q;
        return sigma * 0.25 * diff * diff * diff;
    }
}

fn cubic_spline_grad(r: f32, h: f32) -> f32 {
    let q = r / h;
    if (q >= 2.0 || r < 1e-10) { return 0.0; }
    let sigma = 1.0 / (PI * h * h * h * h);
    if (q < 1.0) {
        return sigma * (-3.0 * q + 2.25 * q * q);
    } else {
        let diff = 2.0 - q;
        return sigma * (-0.75 * diff * diff);
    }
}

@compute @workgroup_size(256)
fn compute_density(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = params.num_particles;
    if (i >= n) { return; }

    let p_i = positions[i].xyz;
    var rho: f32 = params.mass * cubic_spline(0.0, params.h);

    for (var j = 0u; j < n; j = j + 1u) {
        if (j == i) { continue; }
        let p_j = positions[j].xyz;
        let r = distance(p_i, p_j);
        if (r < 2.0 * params.h) {
            rho += params.mass * cubic_spline(r, params.h);
        }
    }

    densities[i] = rho;
}

@compute @workgroup_size(256)
fn compute_pressure(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = params.num_particles;
    if (i >= n) { return; }

    let rho = densities[i];
    let ratio = rho / params.rest_density;
    let b = params.rest_density * params.speed_of_sound * params.speed_of_sound / params.gamma;
    var p: f32 = b * (pow(ratio, params.gamma) - 1.0);

    if (p < 0.0) { p = 0.0; }

    pressures[i] = p;
}

@compute @workgroup_size(256)
fn compute_forces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = params.num_particles;
    if (i >= n) { return; }

    let p_i = positions[i].xyz;
    let v_i = velocities[i].xyz;
    let rho_i = densities[i];
    let press_i = pressures[i];

    var f: vec3<f32> = vec3(0.0, 0.0, 0.0);

    for (var j = 0u; j < n; j = j + 1u) {
        if (j == i) { continue; }
        let p_j = positions[j].xyz;
        let r_vec = p_i - p_j;
        let r = length(r_vec);
        if (r >= 2.0 * params.h || r < 1e-10) { continue; }

        let r_hat = r_vec / r;
        let dw_dr = cubic_spline_grad(r, params.h);

        // Pressure gradient force: symmetric form
        let term = press_i / (rho_i * rho_i) + pressures[j] / (densities[j] * densities[j]);
        f += params.mass * params.mass * term * dw_dr * r_hat;

        // Artificial viscosity (Monaghan)
        let v_rel = v_i - velocities[j].xyz;
        let v_dot_r = dot(v_rel, r_vec);
        if (v_dot_r < 0.0) {
            let rho_bar = 0.5 * (rho_i + densities[j]);
            let mu = (params.h * v_dot_r) / (r * r + EPS2 * params.h * params.h);
            let c = params.speed_of_sound;
            let pi_ij = (-params.alpha_visc * c * mu + params.beta_visc * mu * mu) / rho_bar;
            f += params.mass * params.mass * pi_ij * dw_dr * r_hat;
        }
    }

    forces[i] = vec4(f, 0.0);
}

@compute @workgroup_size(256)
fn compute_integrate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = params.num_particles;
    if (i >= n) { return; }

    let f = forces[i].xyz;
    let v_old = velocities[i].xyz;
    let p_old = positions[i].xyz;

    // Semi-implicit Euler: v += F/m * dt, x += v * dt
    let v_new = v_old + f / params.mass * params.dt;
    let p_new = p_old + v_new * params.dt;

    velocities[i] = vec4(v_new, 0.0);
    positions[i] = vec4(p_new, 0.0);
}

@compute @workgroup_size(256)
fn compute_xsph(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = params.num_particles;
    if (i >= n) { return; }

    let p_i = positions[i].xyz;
    let v_i = velocities[i].xyz;
    let rho_i = densities[i];
    let eps: f32 = 0.1;

    var corr: vec3<f32> = vec3(0.0, 0.0, 0.0);

    for (var j = 0u; j < n; j = j + 1u) {
        if (j == i) { continue; }
        let p_j = positions[j].xyz;
        let r = distance(p_i, p_j);
        if (r >= 2.0 * params.h) { continue; }

        let w = cubic_spline(r, params.h);
        let rho_bar = 0.5 * (rho_i + densities[j]);
        let v_diff = velocities[j].xyz - v_i;
        corr += eps * (params.mass / rho_bar) * v_diff * w;
    }

    velocities[i] = vec4(v_i + corr, 0.0);
}
