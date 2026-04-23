//! Simulation tick budget manager.
//!
//! Entities are classified into simulation zones based on camera distance.
//! The TickBudgetManager runs each heartbeat to reclassify entities and
//! manage the stat pool → agent promotion/demotion pipeline.

/// Simulation zone for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimZone {
    /// Full 60Hz simulation with individual agent tracking
    Active,
    /// 4Hz heartbeat simulation — reduced detail
    Heartbeat,
    /// Statistical pool only — no individual tracking
    Statistical,
}

/// Manages simulation tick budgets per entity.
///
/// # Phase 2 Implementation
///
/// Every heartbeat tick (4Hz):
/// 1. Get camera position from World resource
/// 2. Query all entities with `Transform` component
/// 3. For each: compute distance to camera
/// 4. Compare to `active_radius` and `statistical_radius` thresholds
/// 5. Move entity between `Active`, `Heartbeat`, `Statistical` zones
/// 6. Entities entering `Statistical` zone: remove `Agent` component, add to `StatPool`
/// 7. Entities entering `Active` zone: promote from `StatPool`, add `Agent` component
///
/// The promotion/demotion must be deferred through `EntityCommandBuffer` to avoid
/// invalidating the active query.
pub struct TickBudgetManager {
    pub active_radius: f32,
    pub statistical_radius: f32,
}

impl TickBudgetManager {
    pub fn new(active_radius: f32) -> Self {
        Self {
            active_radius,
            statistical_radius: active_radius * 3.0,
        }
    }

    pub fn zone_for_distance(&self, distance: f32) -> SimZone {
        if distance < self.active_radius {
            SimZone::Active
        } else if distance < self.statistical_radius {
            SimZone::Heartbeat
        } else {
            SimZone::Statistical
        }
    }
}
