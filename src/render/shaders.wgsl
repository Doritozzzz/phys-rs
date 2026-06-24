// ═══════════════════════════════════════════════════════════════════════════
//  Shared uniforms
// ═══════════════════════════════════════════════════════════════════════════

struct Uniforms {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct SkyboxUniforms {
    inv_view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> skybox_uniforms: SkyboxUniforms;

// ═══════════════════════════════════════════════════════════════════════════
//  SPHERE — instanced Phong-lit spheres
// ═══════════════════════════════════════════════════════════════════════════

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) instance_pos: vec3<f32>,
    @location(3) instance_scale: f32,
    @location(4) instance_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) instance_color: vec4<f32>,
    @location(2) view_dir: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let world_pos = input.position * input.instance_scale + input.instance_pos;
    var output: VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(world_pos, 1.0);
    output.world_normal = input.normal;
    output.instance_color = input.instance_color;
    output.view_dir = normalize(-world_pos);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let N = normalize(input.world_normal);
    let L = light_dir;
    let V = normalize(input.view_dir);

    let ambient = 0.05 * input.instance_color.rgb;

    let NdotL = max(dot(N, L), 0.0);
    let diffuse = NdotL * input.instance_color.rgb;

    let H = normalize(L + V);
    let specular = pow(max(dot(N, H), 0.0), 32.0) * vec3<f32>(0.3);

    return vec4<f32>(ambient + diffuse + specular, input.instance_color.a);
}

// ═══════════════════════════════════════════════════════════════════════════
//  LINE — coloured line segments (trails, vectors, grid)
// ═══════════════════════════════════════════════════════════════════════════

struct LineVertexInput {
    @location(5) position: vec3<f32>,
    @location(6) color: vec4<f32>,
}

struct LineVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn line_vs(input: LineVertexInput) -> LineVertexOutput {
    var output: LineVertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn line_fs(input: LineVertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

// ═══════════════════════════════════════════════════════════════════════════
//  SKYBOX — procedural starfield (full-screen triangle)
// ═══════════════════════════════════════════════════════════════════════════

struct SkyboxVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ray_dir: vec3<f32>,
}

@vertex
fn skybox_vs(@builtin(vertex_index) vi: u32) -> SkyboxVertexOutput {
    // Full-screen triangle (no vertex buffer needed).
    // vi 0→(-1,-1), vi 1→(3,-1), vi 2→(-1,3) covering entire NDC rect.
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    let ndc = uv * 2.0 - 1.0;

    // Two-point ray direction: far − near cancels camera translation.
    let far_c  = skybox_uniforms.inv_view_proj * vec4<f32>(ndc.x, ndc.y, 1.0, 1.0);
    let near_c = skybox_uniforms.inv_view_proj * vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);

    var output: SkyboxVertexOutput;
    output.position = vec4<f32>(ndc.x, ndc.y, 1.0, 1.0);
    output.ray_dir  = normalize(far_c.xyz / far_c.w - near_c.xyz / near_c.w);
    return output;
}

// Simple hash for pseudo-random star placement
fn hash21(p: vec2<f32>) -> f32 {
    let h = fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
    return h;
}

fn hash33(p: vec3<f32>) -> f32 {
    let h = fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
    return h;
}

@fragment
fn skybox_fs(input: SkyboxVertexOutput) -> @location(0) vec4<f32> {
    let rd = normalize(input.ray_dir);

    // Star field via multiple octaves of noise
    var star_color: vec3<f32> = vec3<f32>(0.0);

    // Three octaves of stars with different scales and densities
    for (var oct = 0u; oct < 3u; oct = oct + 1u) {
        let scale = vec3<f32>(100.0, 100.0, 100.0) * f32(1u << oct);
        let density = 0.001 * f32(1u << oct);
        let p = rd * scale;
        let cell = floor(p);
        let local = fract(p);

        // Evaluate one random star per cell
        let h = hash33(cell);
        if h < density {
            let star_pos = vec3<f32>(hash33(cell + vec3<f32>(1.0, 0.0, 0.0)), hash33(cell + vec3<f32>(0.0, 1.0, 0.0)), hash33(cell + vec3<f32>(0.0, 0.0, 1.0)));
            let diff = local - star_pos;
            let d2 = dot(diff, diff);
            if d2 < 0.001 {
                let brightness = 1.0 - d2 * 500.0;
                let temp = hash33(cell + vec3<f32>(3.0, 7.0, 11.0)) * 3.0 + 0.5; // 0.5..3.5
                // Star color varies from blue-white to red
                let star_temp = vec3<f32>(1.0, 0.7 + temp * 0.1, 0.5 + temp * 0.15);
                star_color += star_temp * brightness * (3.0 + f32(oct) * 1.5);
            }
        }
    }

    // Very faint nebula-like background variation
    let nebula = 0.001 * (sin(rd.x * 2.0 + rd.y * 3.0 + rd.z * 5.0) * 0.5 + 0.5);

    return vec4<f32>(star_color + vec3<f32>(nebula * 0.05, nebula * 0.02, nebula * 0.08), 1.0);
}
