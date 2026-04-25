use crate::camera::Camera;
use canopy_platform::input::{InputState, KeyCode};
use std::collections::VecDeque;

const FRAME_HISTORY_CAP: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOverlayPane {
    FpsGraph,
    SecondaryCamera,
    EntityBreakdown,
    SystemStats,
    Help,
    Culling,
    Timings,
}

#[derive(Debug, Clone, Default)]
pub struct PerfSystemStats {
    pub cpu_name: String,
    pub cpu_usage_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub gpu_name: String,
    pub gpu_usage_percent: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct SecondaryCameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub pan: glam::Vec3,
    pub camera: Camera,
}

impl SecondaryCameraState {
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.2,
            distance: 12.0,
            pan: glam::Vec3::ZERO,
            camera: Camera::new(45.0, 1.0),
        }
    }

    pub fn update_from_main(&mut self, dt: f64, input: &InputState, main: &Camera) {
        // Orbital controls are only active while the secondary pane is selected.
        let orbit_speed = 1.8_f32;
        let zoom_speed = 12.0_f32;
        let pan_speed = 10.0_f32;

        if input.key_held(KeyCode::Left) {
            self.yaw += orbit_speed * dt as f32;
        }
        if input.key_held(KeyCode::Right) {
            self.yaw -= orbit_speed * dt as f32;
        }
        if input.key_held(KeyCode::Up) {
            self.pitch = (self.pitch + orbit_speed * dt as f32).clamp(-1.2, 1.2);
        }
        if input.key_held(KeyCode::Down) {
            self.pitch = (self.pitch - orbit_speed * dt as f32).clamp(-1.2, 1.2);
        }
        if input.key_held(KeyCode::Q) {
            self.distance = (self.distance - zoom_speed * dt as f32).max(2.0);
        }
        if input.key_held(KeyCode::E) {
            self.distance = (self.distance + zoom_speed * dt as f32).min(150.0);
        }

        let right = main.forward.cross(main.up).normalize_or_zero();
        let flat_forward = glam::Vec3::new(main.forward.x, 0.0, main.forward.z).normalize_or_zero();
        if input.key_held(KeyCode::A) {
            self.pan -= right * pan_speed * dt as f32;
        }
        if input.key_held(KeyCode::D) {
            self.pan += right * pan_speed * dt as f32;
        }
        if input.key_held(KeyCode::W) {
            self.pan += flat_forward * pan_speed * dt as f32;
        }
        if input.key_held(KeyCode::S) {
            self.pan -= flat_forward * pan_speed * dt as f32;
        }

        let target = main.position + main.forward * 8.0 + self.pan;
        let orbit_offset = glam::Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ) * self.distance;

        self.camera.position = target + orbit_offset;
        self.camera.forward = (target - self.camera.position).normalize_or_zero();
        self.camera.up = glam::Vec3::Y;
        self.camera.fov_y_radians = main.fov_y_radians;
        self.camera.aspect = main.aspect;
        self.camera.near = main.near;
        self.camera.far = main.far;
    }
}

impl Default for SecondaryCameraState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PerfToolkitState {
    pub enabled: bool,
    pub active_overlay: Option<ActiveOverlayPane>,

    pub fps_average: f32,
    pub fps_1pct_low: f32,
    pub latency_ms: f32,
    pub fps_history: VecDeque<f32>,

    pub entity_count: usize,
    pub visible_classes: Vec<(String, usize)>,

    pub system_stats: PerfSystemStats,
    pub secondary_camera: SecondaryCameraState,
}

impl Default for PerfToolkitState {
    fn default() -> Self {
        Self {
            enabled: false,
            active_overlay: None,
            fps_average: 0.0,
            fps_1pct_low: 0.0,
            latency_ms: 0.0,
            fps_history: VecDeque::with_capacity(FRAME_HISTORY_CAP),
            entity_count: 0,
            visible_classes: Vec::new(),
            system_stats: PerfSystemStats::default(),
            secondary_camera: SecondaryCameraState::default(),
        }
    }
}

impl PerfToolkitState {
    pub fn update_toggle_state(&mut self, input: &InputState) {
        if input.key_just_pressed(KeyCode::F3) || input.key_just_pressed(KeyCode::Grave) {
            self.enabled = !self.enabled;
            if !self.enabled {
                self.active_overlay = None;
            }
        }

        let debug_modifier_held = input.key_held(KeyCode::F3) || input.key_held(KeyCode::Grave);
        if !self.enabled || !debug_modifier_held {
            return;
        }

        if input.key_just_pressed(KeyCode::G) {
            self.toggle_overlay(ActiveOverlayPane::FpsGraph);
        }
        if input.key_just_pressed(KeyCode::W) {
            self.toggle_overlay(ActiveOverlayPane::SecondaryCamera);
        }
        if input.key_just_pressed(KeyCode::E) {
            self.toggle_overlay(ActiveOverlayPane::EntityBreakdown);
        }
        if input.key_just_pressed(KeyCode::S) {
            self.toggle_overlay(ActiveOverlayPane::SystemStats);
        }

        // Extra panes for engine debugging.
        if input.key_just_pressed(KeyCode::H) {
            self.toggle_overlay(ActiveOverlayPane::Help);
        }
        if input.key_just_pressed(KeyCode::C) {
            self.toggle_overlay(ActiveOverlayPane::Culling);
        }
        if input.key_just_pressed(KeyCode::L) {
            self.toggle_overlay(ActiveOverlayPane::Timings);
        }
    }

    fn toggle_overlay(&mut self, pane: ActiveOverlayPane) {
        if self.active_overlay == Some(pane) {
            self.active_overlay = None;
        } else {
            self.active_overlay = Some(pane);
        }
    }

    pub fn update_frame_metrics(&mut self, dt_seconds: f64) {
        if dt_seconds <= 0.0 {
            return;
        }
        let fps = (1.0 / dt_seconds) as f32;
        self.fps_history.push_back(fps);
        while self.fps_history.len() > FRAME_HISTORY_CAP {
            self.fps_history.pop_front();
        }

        let count = self.fps_history.len() as f32;
        self.fps_average = if count > 0.0 {
            self.fps_history.iter().copied().sum::<f32>() / count
        } else {
            0.0
        };

        let mut samples: Vec<f32> = self.fps_history.iter().copied().collect();
        samples.sort_by(|a, b| a.total_cmp(b));
        let low_count = ((samples.len() as f32) * 0.01).ceil() as usize;
        let low_count = low_count.max(1).min(samples.len());
        let low_slice = &samples[..low_count];
        self.fps_1pct_low = low_slice.iter().copied().sum::<f32>() / low_slice.len() as f32;

        self.latency_ms = (dt_seconds as f32) * 1000.0;
    }

    pub fn update_secondary_camera(&mut self, dt: f64, input: &InputState, main_camera: Option<&Camera>) {
        if !self.enabled || self.active_overlay != Some(ActiveOverlayPane::SecondaryCamera) {
            return;
        }
        if let Some(main) = main_camera {
            self.secondary_camera.update_from_main(dt, input, main);
        }
    }
}

pub fn classify_asset(asset_path: &str) -> String {
    let stem = std::path::Path::new(asset_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    stem.split('_').next().unwrap_or(stem).to_ascii_lowercase()
}

