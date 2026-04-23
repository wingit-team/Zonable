//! `AppStage` — frame execution phases.
//!
//! Systems are tagged with a stage. The engine runs stages in this order
//! every frame:
//!
//! ```text
//! PreUpdate
//!   → input events dispatched
//!   → entity command buffer from previous frame flushed
//! Update
//!   → game logic systems
//!   → Python script systems (via ScriptRunner)
//! PostUpdate
//!   → transform hierarchy propagation
//!   → camera frustum update
//! Render
//!   → draw call submission
//!   → ImGui / debug overlay
//! PostRender
//!   → frame statistics
//!   → telemetry / profiler flush
//! ```

/// Frame execution stage. Lower numeric value = runs first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppStage {
    /// Input handling and command buffer flush.
    PreUpdate = 0,
    /// Main game logic.
    Update = 1,
    /// Transform propagation, culling.
    PostUpdate = 2,
    /// GPU draw call submission.
    Render = 3,
    /// Stats, profiling, cleanup.
    PostRender = 4,
}

impl AppStage {
    pub const ALL: &'static [AppStage] = &[
        AppStage::PreUpdate,
        AppStage::Update,
        AppStage::PostUpdate,
        AppStage::Render,
        AppStage::PostRender,
    ];
}

impl From<AppStage> for canopy_ecs::system::SystemStage {
    fn from(stage: AppStage) -> Self {
        match stage {
            AppStage::PreUpdate => canopy_ecs::system::SystemStage::PreUpdate,
            AppStage::Update => canopy_ecs::system::SystemStage::Update,
            AppStage::PostUpdate => canopy_ecs::system::SystemStage::PostUpdate,
            AppStage::Render => canopy_ecs::system::SystemStage::Render,
            AppStage::PostRender => canopy_ecs::system::SystemStage::PostRender,
        }
    }
}
