struct Uniforms {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

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
