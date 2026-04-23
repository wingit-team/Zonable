//! Flow-based traffic network.
//!
//! # Why flow-based instead of per-agent pathfinding?
//!
//! A city with 200k citizens, each commuting twice daily, would require 400k A* calls.
//! At 60fps that's impossibly expensive. Flow-based traffic treats the road network as
//! a fluid: demand accumulates at sources (homes), supply drains at sinks (workplaces),
//! and flow spreads along edges proportional to capacity minus current load.
//!
//! # Algorithm (Phase 2 Implementation)
//!
//! Each road segment has:
//! - `capacity`: max vehicles/minute at free flow speed
//! - `load`: current vehicles/minute (sum of agents traversing)
//! - `flow`: target flow rate computed by relaxation
//!
//! Each tick:
//! 1. Compute demand at each zone (workers needing to travel)
//! 2. For each source node, push flow outward along edges proportional to
//!    (capacity - load) / distance_to_sink
//! 3. Vehicles pick the highest-flow adjacent segment (greedy, no full path compute)
//! 4. Load = sum of vehicles currently on segment (from active agents)
//!    + estimated statistical load (from StatPools)
//!
//! This converges to a near-optimal flow assignment in 2-3 relaxation iterations.

use ahash::AHashMap;
use glam::Vec3;

/// A node in the road network (intersection or endpoint).
#[derive(Debug, Clone)]
pub struct RoadNode {
    pub id: u32,
    pub position: Vec3,
    /// Adjacent road segment IDs
    pub segments: Vec<u32>,
    /// Incoming demand (vehicles wanting to leave this node)
    pub demand: f32,
    /// Incoming supply (vehicles wanting to arrive here — jobs, shops, etc.)
    pub supply: f32,
}

/// A directed road segment between two nodes.
#[derive(Debug, Clone)]
pub struct RoadSegment {
    pub id: u32,
    pub from_node: u32,
    pub to_node: u32,
    /// Metres
    pub length: f32,
    /// Max speed (m/s)
    pub speed_limit: f32,
    /// Max vehicles per minute at free flow
    pub capacity: f32,
    /// Current load (vehicles per minute)
    pub load: f32,
    /// Computed flow rate (used by agents to make routing decisions)
    pub flow: f32,
}

impl RoadSegment {
    /// Congestion ratio [0.0 = free flow, 1.0 = fully saturated, >1.0 = gridlock]
    pub fn congestion(&self) -> f32 {
        if self.capacity > 0.0 { self.load / self.capacity } else { f32::INFINITY }
    }

    /// Travel time in seconds (BPR function — standard traffic engineering model)
    pub fn travel_time(&self) -> f32 {
        let free_flow_time = self.length / self.speed_limit;
        let cong = self.congestion();
        // Bureau of Public Roads volume-delay function: t = t0 * (1 + 0.15 * (v/c)^4)
        free_flow_time * (1.0 + 0.15 * cong.powi(4))
    }
}

pub struct TrafficNetwork {
    pub nodes: AHashMap<u32, RoadNode>,
    pub segments: AHashMap<u32, RoadSegment>,
}

impl TrafficNetwork {
    pub fn new() -> Self {
        Self {
            nodes: AHashMap::new(),
            segments: AHashMap::new(),
        }
    }

    /// Run one relaxation iteration of the flow field.
    /// Call 2-3 times per sim tick for convergence.
    pub fn relax_flow(&mut self) {
        // TODO Phase 2:
        // For each segment, compute desired flow based on demand gradient
        // between from_node.demand and to_node.supply, weighted by
        // remaining capacity. This is a simplified version of Wardrop's
        // user equilibrium principle.
        for seg in self.segments.values_mut() {
            let utilization = (seg.load / seg.capacity.max(1.0)).clamp(0.0, 2.0);
            seg.flow = seg.capacity * (1.0 - utilization * 0.5);
        }
    }

    /// Find the best adjacent segment to move toward `destination` from `current_node`.
    /// Agents call this each tick to make routing decisions without full path computation.
    pub fn best_next_segment(&self, current_node: u32, destination_node: u32) -> Option<u32> {
        let node = self.nodes.get(&current_node)?;
        // Greedy: pick the segment with highest (flow - congestion_penalty) toward destination
        node.segments.iter()
            .filter_map(|&seg_id| self.segments.get(&seg_id).map(|s| (seg_id, s)))
            .max_by(|(_, a), (_, b)| {
                let score_a = a.flow / a.travel_time().max(0.001);
                let score_b = b.flow / b.travel_time().max(0.001);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| id)
    }
}

impl Default for TrafficNetwork {
    fn default() -> Self { Self::new() }
}
