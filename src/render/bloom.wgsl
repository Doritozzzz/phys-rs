// ═══════════════════════════════════════════════════════════════════════════
//  Bloom post-processing shaders (S2.11)
//
//  Pipeline:
//    1. Extract bright pixels from HDR → half-res bloom texture
//    2. Gaussian blur horizontal  (9-tap, ping-pong)
//    3. Gaussian blur vertical    (9-tap, ping-pong)
//    4. Composite HDR + bloom → ACES tonemap → LDR output
// ═══════════════════════════════════════════════════════════════════════════

// ── Shared: full-screen triangle vertex shader ─────────────────────────────

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn fullscreen_vs(@builtin(vertex_index) vi: u32) -> FullscreenVertexOutput {
    // Full-screen triangle via vertex index.
    // vi 0→(0,0), vi 1→(2,0), vi 2→(0,2) in UV, mapped to NDC -1..3
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    let ndc = uv * 2.0 - 1.0;
    var output: FullscreenVertexOutput;
    output.position = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
    output.uv = uv;
    return output;
}

// ── Pass 1: Extract bright pixels + downsample 2× ─────────────────────────

@group(0) @binding(0) var hdr_sampler: sampler;
@group(0) @binding(1) var hdr_texture: texture_2d<f32>;

// Bloom parameters packed into a uniform buffer
struct BloomUniforms {
    threshold: f32,
    intensity: f32,
}
@group(0) @binding(2) var<uniform> bloom_params: BloomUniforms;

@fragment
fn bloom_extract_fs(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(hdr_texture, hdr_sampler, input.uv);
    // NaN/Inf guard: clamp to valid f16 range, kill NaN/negative
    let safe = clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(65504.0));
    let luminance = dot(safe, vec3<f32>(0.2126, 0.7152, 0.0722));
    let excess = max(luminance - bloom_params.threshold, 0.0);
    let factor = select(0.0, excess / luminance, luminance > 0.001);
    return vec4<f32>(safe * factor, 1.0);
}

// ── Pass 2/3: Separable 9-tap gaussian blur ───────────────────────────────

@group(0) @binding(0) var bloom_sampler: sampler;
@group(0) @binding(1) var bloom_texture: texture_2d<f32>;

// Horizontal blur
@fragment
fn gaussian_blur_h_fs(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv;
    let size = vec2<f32>(textureDimensions(bloom_texture));
    let texel = 1.0 / size;

    // 9-tap gaussian: weights sum ≈ 1.0
    let offsets = array<f32, 9>(-4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0);
    let weights = array<f32, 9>(0.0162, 0.0646, 0.1207, 0.1806, 0.2078, 0.1806, 0.1207, 0.0646, 0.0162);

    var result: vec3<f32> = vec3<f32>(0.0);
    for (var i = 0u; i < 9u; i = i + 1u) {
        let offset = offsets[i] * texel.x;
        let sample_uv = vec2<f32>(uv.x + offset, uv.y);
        result += textureSample(bloom_texture, bloom_sampler, sample_uv).rgb * weights[i];
    }
    return vec4<f32>(max(result, vec3<f32>(0.0)), 1.0);
}

// Vertical blur
@fragment
fn gaussian_blur_v_fs(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv;
    let size = vec2<f32>(textureDimensions(bloom_texture));
    let texel = 1.0 / size;

    let offsets = array<f32, 9>(-4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0);
    let weights = array<f32, 9>(0.0162, 0.0646, 0.1207, 0.1806, 0.2078, 0.1806, 0.1207, 0.0646, 0.0162);

    var result: vec3<f32> = vec3<f32>(0.0);
    for (var i = 0u; i < 9u; i = i + 1u) {
        let offset = offsets[i] * texel.y;
        let sample_uv = vec2<f32>(uv.x, uv.y + offset);
        result += textureSample(bloom_texture, bloom_sampler, sample_uv).rgb * weights[i];
    }
    return vec4<f32>(max(result, vec3<f32>(0.0)), 1.0);
}

// ── Pass 4: Composite HDR + bloom → ACES tonemap → LDR ────────────────────

@group(0) @binding(0) var composite_sampler: sampler;
@group(0) @binding(1) var composite_hdr_texture: texture_2d<f32>;
@group(0) @binding(2) var composite_bloom_texture: texture_2d<f32>;
@group(0) @binding(3) var<uniform> composite_params: BloomUniforms;

// ACES filmic tonemap (Narkowicz 2015)
fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3(0.0), vec3(1.0));
}

// ── Debug: passthrough HDR → LDR (no ACES, no bloom) ────────────────────

@group(0) @binding(0) var pass_sampler: sampler;
@group(0) @binding(1) var pass_texture: texture_2d<f32>;

@fragment
fn passthrough_fs(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(pass_texture, pass_sampler, input.uv).rgb;
    return vec4<f32>(clamp(color, vec3(0.0), vec3(1.0)), 1.0);
}

@fragment
fn composite_fs(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Final: HDR + bloom * intensity → ACES
    let hdr = textureSample(composite_hdr_texture, composite_sampler, input.uv).rgb;
    let bloom = textureSample(composite_bloom_texture, composite_sampler, input.uv).rgb;
    let combined = hdr + bloom * composite_params.intensity;
    let tonemapped = aces_tonemap(combined);
    return vec4<f32>(tonemapped, 1.0);
}
