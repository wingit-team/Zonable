//! Agent — full-fidelity simulated entity (citizen, vehicle).

use canopy_ecs::entity::Entity;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Citizen,
    Vehicle,
    Emergency,
    Tourist,
}

/// A fully simulated agent (individual citizen or vehicle).
/// These only exist for entities within the active simulation radius.
/// Outside that radius they collapse into `StatPool` buckets.
#[derive(Debug, Clone)]
pub struct Agent {
    pub entity: Entity,
    pub kind: AgentKind,
    pub position: Vec3,
    pub velocity: Vec3,
    pub home_zone: u32,
    pub work_zone: u32,
    pub happiness: f32,
    pub employment_status: EmploymentStatus,
    /// Which road segment this agent is currently traversing (traffic)
    pub current_road_segment: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmploymentStatus {
    Employed,
    Unemployed,
    Retired,
    Student,
}
