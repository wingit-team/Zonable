use canopy_ecs::world::World;
use canopy_platform::input::InputState;

#[derive(Debug, Clone)]
pub struct OrbitCameraState {
    pub target: glam::Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for OrbitCameraState {
    fn default() -> Self {
        Self {
            target: glam::Vec3::ZERO,
            distance: 5.0,
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.45,
        }
    }
}

pub fn orbit_camera_system(world: &mut World, _dt: f64) {
    let input = match world.get_resource::<InputState>() {
        Some(i) => i.clone(),
        None => return,
    };

    // Acquire mutable state and update based on input
    let (target, distance, yaw, pitch) = {
        let mut state = match world.get_resource_mut::<OrbitCameraState>() {
            Some(s) => s,
            None => return,
        };
        // Mouse motion always orbits — no button hold required.
        let mouse = input.mouse_delta();
        if mouse.x != 0.0 || mouse.y != 0.0 {
            state.yaw -= mouse.x * 0.005;
            state.pitch = (state.pitch - mouse.y * 0.005).clamp(-1.54, 1.54);
        }
        // Scroll to zoom
        let scroll = input.scroll_delta();
        state.distance = (state.distance - scroll.y * 0.05).max(1.0).max(0.1);
        // Copy values for later use
        (state.target, state.distance, state.yaw, state.pitch)
    };

    // Acquire mutable camera reference
    let camera = match world.get_resource_mut::<canopy_renderer::Camera>() {
        Some(c) => c,
        None => return,
    };

    // Calculate camera position based on spherical coordinates
    let offset = glam::Vec3::new(
        distance * pitch.cos() * yaw.cos(),
        distance * pitch.sin(),
        distance * pitch.cos() * yaw.sin(),
    );
    camera.position = target + offset;
    camera.forward = (target - camera.position).normalize_or_zero();
    camera.up = glam::Vec3::Y; // Keep up as Y for simplicity
}

