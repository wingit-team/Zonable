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

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) clip_xy: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );

    var out: VsOut;
    out.clip_xy = positions[vertex_index];
    out.pos = vec4<f32>(out.clip_xy, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let near_clip = vec4<f32>(in.clip_xy, 0.0, 1.0);
    let far_clip = vec4<f32>(in.clip_xy, 1.0, 1.0);

    let near_world_h = camera.inv_view_proj * near_clip;
    let far_world_h = camera.inv_view_proj * far_clip;
    let near_world = near_world_h.xyz / near_world_h.w;
    let far_world = far_world_h.xyz / far_world_h.w;
    let ray_dir = normalize(far_world - near_world);

    let up_amount = clamp(ray_dir.y * 0.5 + 0.5, 0.0, 1.0);
    var sky = mix(camera.sky_horizon_color.rgb, camera.sky_top_color.rgb, up_amount);

    // Strong visible sun disk in sky.
    let sun_dir = normalize(-camera.sun_direction.xyz);
    let sun_dot = max(dot(ray_dir, sun_dir), 0.0);
    let sun_core = smoothstep(0.9985, 1.0, sun_dot);
    let sun_halo = smoothstep(0.965, 1.0, sun_dot) * 0.25;
    sky += vec3<f32>(1.3, 1.15, 0.9) * (sun_core + sun_halo);

    return vec4<f32>(sky, 1.0);
}

