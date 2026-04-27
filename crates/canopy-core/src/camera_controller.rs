use canopy_ecs::world::World;
use canopy_platform::input::{InputState, KeyCode};

#[derive(Debug, Clone)]
pub struct FreeFlyCameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub move_speed: f32,
    pub look_sensitivity: f32,
}

impl Default for FreeFlyCameraState {
    fn default() -> Self {
        Self {
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.45,
            move_speed: 8.0,
            look_sensitivity: 0.0018,
        }
    }
}

pub fn free_fly_camera_system(world: &mut World, dt: f64) {
    let input = match world.get_resource::<InputState>() {
        Some(i) => i.clone(),
        None => return,
    };

    let state = world
        .get_resource_mut::<FreeFlyCameraState>()
        .map(|s| {
            let mouse = input.mouse_delta();
            s.yaw -= mouse.x * s.look_sensitivity;
            s.pitch = (s.pitch - mouse.y * s.look_sensitivity).clamp(-1.54, 1.54);
            s.clone()
        })
        .unwrap_or_default();

    let Some(camera) = world.get_resource_mut::<canopy_renderer::Camera>() else {
        return;
    };

    let (sin_yaw, cos_yaw) = state.yaw.sin_cos();
    let (sin_pitch, cos_pitch) = state.pitch.sin_cos();
    let forward = glam::Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw)
        .normalize_or_zero();
    let right = forward.cross(glam::Vec3::Y).normalize_or_zero();

    let mut direction = glam::Vec3::ZERO;
    if input.key_held(KeyCode::W) {
        direction += forward;
    }
    if input.key_held(KeyCode::S) {
        direction -= forward;
    }
    if input.key_held(KeyCode::A) {
        direction -= right;
    }
    if input.key_held(KeyCode::D) {
        direction += right;
    }
    if input.key_held(KeyCode::Space) {
        direction += glam::Vec3::Y;
    }
    if input.key_held(KeyCode::LeftShift) || input.key_held(KeyCode::RightShift) {
        direction -= glam::Vec3::Y;
    }

    let direction = direction.normalize_or_zero();
    camera.position += direction * (state.move_speed * dt as f32);
    camera.forward = forward;
    camera.up = glam::Vec3::Y;
}

