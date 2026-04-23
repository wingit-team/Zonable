//! Simulation event system — typed pub/sub for game events.

use glam::Vec3;

/// Simulation events that can be published and subscribed to.
#[derive(Debug, Clone)]
pub enum SimEvent {
    Earthquake(EarthquakeEvent),
    Flood { zone_id: u32, severity: f32 },
    Fire { zone_id: u32, spread_rate: f32 },
    PowerOutage { zone_ids: Vec<u32>, duration_ticks: u32 },
    PolicyChange { policy_id: u32, old_value: f32, new_value: f32 },
    ZoneCreated { zone_id: u32 },
    ZoneDestroyed { zone_id: u32 },
    BuildingCompleted { entity_id: u64 },
    BuildingDemolished { entity_id: u64 },
}

/// A disaster event with epicenter and blast radius.
pub trait DisasterEvent: Send + Sync {
    fn epicenter(&self) -> Vec3;
    fn affected_radius_meters(&self) -> f32;
    fn severity(&self) -> f32; // 0.0 .. 1.0
}

#[derive(Debug, Clone)]
pub struct EarthquakeEvent {
    pub epicenter: Vec3,
    pub magnitude: f32, // Richter scale 1-10
    pub depth_km: f32,
}

impl DisasterEvent for EarthquakeEvent {
    fn epicenter(&self) -> Vec3 { self.epicenter }
    fn affected_radius_meters(&self) -> f32 {
        // Empirical: M6 → ~10km, M7 → ~50km, M8 → ~200km
        10.0f32.powf(self.magnitude - 5.0) * 1000.0
    }
    fn severity(&self) -> f32 { ((self.magnitude - 1.0) / 9.0).clamp(0.0, 1.0) }
}

/// Simple typed event bus. Publishers push events; subscribers receive a snapshot
/// at the next heartbeat tick.
///
/// Phase 2: replace with a proper lock-free MPMC queue per event type.
pub struct EventBus {
    pending: Vec<SimEvent>,
    /// Handlers registered for next dispatch
    handlers: Vec<Box<dyn Fn(&SimEvent) + Send + Sync>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { pending: Vec::new(), handlers: Vec::new() }
    }

    pub fn publish(&mut self, event: SimEvent) {
        self.pending.push(event);
    }

    pub fn subscribe(&mut self, handler: impl Fn(&SimEvent) + Send + Sync + 'static) {
        self.handlers.push(Box::new(handler));
    }

    /// Dispatch all pending events to subscribers. Called each heartbeat tick.
    pub fn flush(&mut self) {
        for event in &self.pending {
            for handler in &self.handlers {
                handler(event);
            }
        }
        self.pending.clear();
    }
}

impl Default for EventBus {
    fn default() -> Self { Self::new() }
}
