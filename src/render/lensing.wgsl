// ═══════════════════════════════════════════════════════════════════════════
//  Gravitational lensing screen-space distortion shader (S2.12)
//
//  Iterates over lens array, computes per-pixel deflection toward each lens
//  center, re-samples HDR texture at distorted UV.
// ═══════════════════════════════════════════════════════════════════════════

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn fullscreen_vs(@builtin(vertex_index) vi: u32) -> FullscreenVertexOutput {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    let ndc = uv * 2.0 - 1.0;
    var output: FullscreenVertexOutput;
    output.position = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
    output.uv = uv;
    return output;
}

// ── Lens data ──────────────────────────────────────────────────────────────

struct LensData {
    uv: vec2<f32>,
    mass: f32,
}

@group(0) @binding(0) var hdr_sampler: sampler;
@group(0) @binding(1) var hdr_texture: texture_2d<f32>;
@group(0) @binding(2) var<storage, read> lenses: array<LensData>;
@group(0) @binding(3) var<uniform> lens_params: vec4<f32>;
// lens_params.x = count (as f32)
// lens_params.y = strength (visual multiplier)
// lens_params.z = mass_scale (e.g. 1e-35 to normalize kg)

@fragment
fn lensing_fs(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let count = u32(lens_params.x);
    let strength = lens_params.y;
    let mass_scale = lens_params.z;

    var sample_uv = input.uv;

    for (var i = 0u; i < count; i = i + 1u) {
        let lens = lenses[i];
        let delta = input.uv - lens.uv;
        let dist_sq = dot(delta, delta);
        let softened = max(dist_sq, 1e-8);
        let deflection = strength * lens.mass * mass_scale / softened;
        sample_uv = sample_uv + delta * deflection;
    }

    // Clamp to avoid sampling outside texture
    let clamped = clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    let color = textureSample(hdr_texture, hdr_sampler, clamped);
    return vec4<f32>(color.rgb, 1.0);
}
