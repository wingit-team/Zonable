//! `canopy-sim` — Simulation layer.
//!
//! # Phase 2 Implementation Plan
//!
//! ## Tick Budget Manager
//! Entities are bucketed by distance from camera:
//! - Zone 0 (< active_radius): Full agent simulation at 60Hz
//! - Zone 1 (active_radius .. 2×): Reduced agent detail at 4Hz heartbeat
//! - Zone 2 (2× ..): Statistical population pools only
//!
//! The `TickBudgetManager` uses the `World`'s archetype system to move entities
//! between `ActiveAgent`, `HeartbeatAgent`, and `StatAgent` archetypes based on
//! camera distance every heartbeat tick.
//!
//! ## Traffic Flow
//! Road network is a directed graph. Instead of per-agent pathfinding (O(n) A* calls),
//! we run a flow field: a heat-map of demand/supply on each road segment, updated
//! every sim tick via relaxation. Vehicles choose the highest-flow adjacent edge.
//! This is inspired by fluid dynamics (pressure-driven flow) and scales to city-level
//! traffic without per-vehicle path computation.
//!
//! ## Economic Simulation
//! `EconomyLedger` tracks supply/demand registers per (zone_type, resource_type) pair.
//! Prices emerge from supply/demand curves. Workers route to jobs via zone-level
//! demand aggregates, not per-worker pathfinding.

pub mod agent;
pub mod budget;
pub mod economy;
pub mod events;
pub mod traffic;

pub use agent::{Agent, AgentKind};
pub use budget::{SimZone, TickBudgetManager};
pub use economy::EconomyLedger;
pub use events::{DisasterEvent, EarthquakeEvent, EventBus, SimEvent};
pub use traffic::TrafficNetwork;
