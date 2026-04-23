//! Per-frame data passed to all systems.

use std::time::{Duration, Instant};

/// Data available to every system each frame.
///
/// Passed by reference so systems can read timing, input, camera info, etc.
/// without storing pointers to engine subsystems.
#[derive(Debug, Clone)]
pub struct FrameData {
    /// Delta time since last frame, in seconds. Use this for all time-dependent
    /// calculations, NOT raw Duration, to ensure frame-rate independence.
    pub dt: f64,
    /// Fixed dt for simulation systems (1 / target_tick_hz). Use for physics/sim.
    pub fixed_dt: f64,
    /// Monotonically increasing frame index (u64 — wraps after ~584 years at 60fps).
    pub frame_index: u64,
    /// Engine uptime at the start of this frame.
    pub elapsed: Duration,
    /// Timestamp of the start of this frame. Used for profiling.
    pub frame_start: Instant,
    /// Whether this is a "heartbeat" tick (4Hz systems should check this).
    pub is_heartbeat_tick: bool,
    /// Current simulation tick index (counts heartbeats, not full frames).
    pub sim_tick: u64,
}

impl FrameData {
    pub fn first_frame() -> Self {
        Self {
            dt: 1.0 / 60.0,
            fixed_dt: 1.0 / 60.0,
            frame_index: 0,
            elapsed: Duration::ZERO,
            frame_start: Instant::now(),
            is_heartbeat_tick: true,
            sim_tick: 0,
        }
    }
}

/// Tracks frame timing state across the engine main loop.
pub struct FrameTimer {
    pub start_time: Instant,
    pub last_frame: Instant,
    pub frame_index: u64,
    pub target_tick_hz: u32,
    pub heartbeat_hz: u32,
    /// Accumulated time for heartbeat tick tracking
    heartbeat_accumulator: f64,
    pub sim_tick: u64,
}

impl FrameTimer {
    pub fn new(target_tick_hz: u32, heartbeat_hz: u32) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame: now,
            frame_index: 0,
            target_tick_hz,
            heartbeat_hz,
            heartbeat_accumulator: 0.0,
            sim_tick: 0,
        }
    }

    /// Advance to the next frame. Returns the `FrameData` for this frame.
    pub fn tick(&mut self) -> FrameData {
        let now = Instant::now();
        let raw_dt = now.duration_since(self.last_frame).as_secs_f64();
        // Clamp to avoid spiral of death after a hickup (debugger pause etc.)
        let dt = raw_dt.min(0.1);

        self.heartbeat_accumulator += dt;
        let heartbeat_interval = 1.0 / self.heartbeat_hz as f64;
        let is_heartbeat = if self.heartbeat_accumulator >= heartbeat_interval {
            self.heartbeat_accumulator -= heartbeat_interval;
            self.sim_tick += 1;
            true
        } else {
            false
        };

        let data = FrameData {
            dt,
            fixed_dt: 1.0 / self.target_tick_hz as f64,
            frame_index: self.frame_index,
            elapsed: now.duration_since(self.start_time),
            frame_start: now,
            is_heartbeat_tick: is_heartbeat,
            sim_tick: self.sim_tick,
        };

        self.last_frame = now;
        self.frame_index += 1;
        data
    }
}
