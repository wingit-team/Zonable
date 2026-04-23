//! Audio mixer — cpal-based output.
//!
//! Phase 2: spatial 3D audio, reverb zones, music layers.

/// Audio mixer managing active sound instances.
pub struct AudioMixer {
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    // Phase 2: cpal output stream, active voice list, spatial audio graph
}

impl AudioMixer {
    pub fn new() -> Self {
        Self { master_volume: 1.0, sfx_volume: 1.0, music_volume: 0.8 }
    }

    /// Initialize cpal output stream. Call once at engine startup.
    pub fn init(&self) -> Result<(), String> {
        // Phase 2: create cpal host, select output device, build output stream
        tracing::info!("AudioMixer: init (stub — Phase 2)");
        Ok(())
    }
}

impl Default for AudioMixer {
    fn default() -> Self { Self::new() }
}
