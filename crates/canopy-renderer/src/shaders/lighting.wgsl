// Deferred Lighting Pass Shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
    inv_view_proj: mat4x4<f32>, // Added for reconstructing position from depth
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var t_normal: texture_2d<f32>;
@group(1) @binding(2) var t_material: texture_2d<f32>;
@group(1) @binding(3) var t_depth: texture_depth_2d;
@group(1) @binding(4) var s_gbuffer: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    // Generate a fullscreen triangle
    var out: VertexOutput;
    let x = f32((in_vertex_index << 1u) & 2u);
    let y = f32(in_vertex_index & 2u);
    out.uv = vec2<f32>(x, 1.0 - y);
    out.clip_position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return out;
}

fn reconstruct_world_position(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    // Reversed-Z: depth is already in [0, 1]. Clip space is [x, y, z, w].
    let clip_space_pos = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let world_space_pos = camera.inv_view_proj * clip_space_pos;
    return world_space_pos.xyz / world_space_pos.w;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // In wgpu, textureSample is not allowed for unfilterable depth textures. We use textureLoad.
    // textureLoad requires integer coordinates.
    let dimensions = textureDimensions(t_depth);
    let tex_coords = vec2<i32>(in.uv * vec2<f32>(dimensions));
    
    let depth = textureLoad(t_depth, tex_coords, 0);
    
    // Check for skybox/clear depth. Reversed-Z means far plane is 0.0.
    if (depth <= 0.00001) {
        return vec4<f32>(0.5, 0.7, 1.0, 1.0); // Simple sky color
    }
    
    let albedo = textureLoad(t_albedo, tex_coords, 0).rgb;
    let normal_encoded = textureLoad(t_normal, tex_coords, 0).xyz;
    let normal = normalize(normal_encoded * 2.0 - 1.0);
    let material = textureLoad(t_material, tex_coords, 0);
    
    let world_pos = reconstruct_world_position(in.uv, depth);
    
    // Simple directional lighting for now
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 0.5));
    let NdotL = max(dot(normal, light_dir), 0.0);
    
    let ambient = albedo * 0.1;
    let diffuse = albedo * NdotL;
    
    let color = ambient + diffuse;
    
    // Simple tonemapping (Reinhard)
    let mapped = color / (color + vec3<f32>(1.0));
    
    return vec4<f32>(mapped, 1.0);
}
