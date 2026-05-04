@group(0) @binding(0) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> velocities: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> densities: array<f32>;
@group(0) @binding(3) var<storage, read_write> forces: array<vec4<f32>>;

const PI: f32 = 3.14159265359;
const MASS: f32 = 1.0; 
const H: f32 = 1.0; 

fn cubic_spline(r: f32, h: f32) -> f32 {
    let q = r / h;
    if (q >= 2.0) { return 0.0; }
    let sigma = 1.0 / (PI * pow(h, 3.0));
    if (q < 1.0) {
        return sigma * (1.0 - 1.5 * q * q + 0.75 * q * q * q);
    } else {
        let diff = 2.0 - q;
        return sigma * 0.25 * diff * diff * diff;
    }
}

@compute @workgroup_size(64)
fn compute_density(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let num_particles = arrayLength(&positions);
    if (i >= num_particles) { return; }

    let p_i = positions[i].xyz;
    var density = 0.0;

    for (var j = 0u; j < num_particles; j = j + 1u) {
        let p_j = positions[j].xyz;
        let r = distance(p_i, p_j);
        density += MASS * cubic_spline(r, H);
    }

    densities[i] = density;
}

@compute @workgroup_size(64)
fn compute_forces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let num_particles = arrayLength(&positions);
    if (i >= num_particles) { return; }

    forces[i] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
