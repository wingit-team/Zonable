// Forward Pass Shader (for transparents/specials)

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
    inv_view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
    sky_top_color: vec4<f32>,
    sky_horizon_color: vec4<f32>,
    cel_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @location(4) uv1: vec2<f32>,
    @location(5) color: vec4<f32>,
};

struct InstanceInput {
    @location(6) model_matrix_0: vec4<f32>,
    @location(7) model_matrix_1: vec4<f32>,
    @location(8) model_matrix_2: vec4<f32>,
    @location(9) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) view_distance: f32,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.uv = model.uv0;
    out.color = model.color;

    let normal_matrix = mat3x3<f32>(
        model_matrix[0].xyz,
        model_matrix[1].xyz,
        model_matrix[2].xyz,
    );
    out.world_normal = normalize(normal_matrix * model.normal);

    let world_pos = model_matrix * vec4<f32>(model.position, 1.0);
    out.view_distance = distance(world_pos.xyz, camera.position.xyz);
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var s_albedo: sampler;

struct MaterialUniforms {
    base_color: vec4<f32>,
    roughness: f32,
    metallic: f32,
    padding1: f32,
    padding2: f32,
}
@group(1) @binding(2) var<uniform> mat_uniforms: MaterialUniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_albedo, s_albedo, in.uv);
    let albedo = tex_color * mat_uniforms.base_color;

    // Cel-shaded sun lighting with default angled directional light.
    let light_dir = normalize(-camera.sun_direction.xyz);
    let ndotl = max(dot(normalize(in.world_normal), light_dir), 0.0);
    let steps = max(camera.cel_params.x, 1.0);
    let cel = floor(ndotl * steps) / steps;
    let ambient = albedo.rgb * 0.35;
    let diffuse = albedo.rgb * (0.25 + cel * 0.9);
    let lit = ambient + diffuse;

    let fog_density = max(camera.fog_params.x, 0.0);
    let fog_start = max(camera.fog_params.y, 0.0);
    let fog_distance = max(in.view_distance - fog_start, 0.0);
    let fog_factor = 1.0 - exp(-fog_density * fog_distance);
    let fogged = mix(lit, camera.fog_color.rgb, fog_factor);

    let mapped = fogged / (fogged + vec3<f32>(1.0));

    return vec4<f32>(mapped, 1.0);
}
