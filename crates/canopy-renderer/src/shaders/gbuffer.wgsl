// GBuffer Pass Shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>, // w unused
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
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
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

    // Normal matrix is roughly the upper 3x3 of the model matrix (assuming uniform scale)
    let normal_matrix = mat3x3<f32>(
        model_matrix[0].xyz,
        model_matrix[1].xyz,
        model_matrix[2].xyz,
    );
    out.world_normal = normalize(normal_matrix * model.normal);

    let world_pos = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_pos.xyz;
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

struct GBufferOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) material: vec4<f32>, // R: Roughness, G: Metallic
}

// Material Bind Group
@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var s_albedo: sampler;
// Add other textures later as needed

struct MaterialUniforms {
    base_color: vec4<f32>,
    roughness: f32,
    metallic: f32,
    padding1: f32,
    padding2: f32,
}
@group(1) @binding(2) var<uniform> mat_uniforms: MaterialUniforms;

@fragment
fn fs_main(in: VertexOutput) -> GBufferOutput {
    var out: GBufferOutput;
    
    let tex_color = textureSample(t_albedo, s_albedo, in.uv);
    out.albedo = tex_color * mat_uniforms.base_color * in.color;
    
    // Simple normal output (no normal mapping yet)
    out.normal = vec4<f32>(normalize(in.world_normal) * 0.5 + 0.5, 1.0);
    
    out.material = vec4<f32>(mat_uniforms.roughness, mat_uniforms.metallic, 0.0, 1.0);
    
    return out;
}
