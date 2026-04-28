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
    main_position: vec4<f32>,
    main_forward: vec4<f32>,
    main_right: vec4<f32>,
    main_up: vec4<f32>,
    main_fov_aspect: vec4<f32>,
    pass_flags: vec4<f32>,
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
    @location(4) world_pos: vec3<f32>,
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
    out.world_pos = world_pos.xyz;
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

    // Smooth normal for lighting
    let smooth_normal = normalize(in.world_normal);
    let light_dir = normalize(-camera.sun_direction.xyz);
    let ndotl = max(dot(smooth_normal, light_dir), 0.0);
    let steps = max(camera.cel_params.x, 1.0);
    let cel = floor(ndotl * steps) / steps;
    let ambient = albedo.rgb * 0.08;
    let sun_tint = vec3<f32>(1.15, 1.0, 0.82);
    let diffuse = albedo.rgb * sun_tint * (0.02 + cel * 1.45);
    let lit = ambient + diffuse;

    let fog_density = max(camera.fog_params.x, 0.0);
    let fog_start = max(camera.fog_params.y, 0.0);
    let fog_distance = max(in.view_distance - fog_start, 0.0);
    let fog_factor = 1.0 - exp(-fog_density * fog_distance);
    let fogged = mix(lit, camera.fog_color.rgb, fog_factor);

    let mapped = fogged / (fogged + vec3<f32>(1.0));

    // Secondary pass flags (debug camera) - we need the faceted normal for this
    if camera.pass_flags.x > 0.5 {
        let geom_base = normalize(cross(dpdx(in.world_pos), dpdy(in.world_pos)));
        let view_to_render = normalize(camera.position.xyz - in.world_pos);
        let geom_normal = select(-geom_base, geom_base, dot(geom_base, view_to_render) >= 0.0);

        let to_frag = in.world_pos - camera.main_position.xyz;
        let depth = dot(to_frag, camera.main_forward.xyz);
        if depth < camera.main_fov_aspect.z || depth > camera.main_fov_aspect.w {
            discard;
        }

        let x = abs(dot(to_frag, camera.main_right.xyz));
        let y = abs(dot(to_frag, camera.main_up.xyz));
        let half_y = camera.main_fov_aspect.x * depth;
        let half_x = camera.main_fov_aspect.y * depth;
        if x > half_x || y > half_y {
            discard;
        }

        let view_to_main = normalize(camera.main_position.xyz - in.world_pos);
        if dot(geom_normal, view_to_main) <= 0.0 {
            discard;
        }
    }

    return vec4<f32>(mapped, 1.0);
}
