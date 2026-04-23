//! System trait and stage ordering.
//!
//! # Systems
//!
//! A `System` in Canopy is anything that implements `fn run(&mut self, world: &mut World, dt: f64)`.
//! Systems are registered into a `SystemScheduler` with a `SystemStage` tag.
//!
//! # Stages
//!
//! Frame execution order:
//!
//! ```text
//! [PreUpdate]  → input handling, event dispatch
//! [Update]     → game logic, simulation tick
//! [PostUpdate] → late-frame transforms, hierarchy propagation
//! [Render]     → submit draw calls to renderer
//! [PostRender] → cleanup, stats collection
//! ```
//!
//! # Parallelism (Phase 2 plan)
//!
//! Each stage runs systems sequentially in Phase 1. In Phase 2, `SystemScheduler`
//! will analyze `WorldAccess` descriptors and build a DAG of non-conflicting
//! systems, dispatching independent systems via `rayon::join`. Systems that share
//! write access to the same component type will be serialized.

use crate::world::World;
use std::time::Duration;

/// Frame execution stage. Systems in lower stages run before higher stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SystemStage {
    PreUpdate = 0,
    Update = 1,
    PostUpdate = 2,
    Render = 3,
    PostRender = 4,
}

/// Core system trait. Every game system (Rust-side) implements this.
///
/// Python systems implement the `System` class in `canopy-script`, which
/// dispatches through the `ScriptRunner` back to this trait.
pub trait System: Send + Sync + 'static {
    /// Called once per frame (or per sim tick for heartbeat systems).
    fn run(&mut self, world: &mut World, dt: f64);

    /// Which stage should this system run in?
    fn stage(&self) -> SystemStage {
        SystemStage::Update
    }

    /// Human-readable name for debugging and profiling.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Type-erased boxed system.
pub type BoxedSystem = Box<dyn System>;

/// Wraps a plain function as a system for ergonomic registration.
///
/// ```rust
/// app.add_system(SystemStage::Update, FnSystem::new(|world, dt| {
///     // ...
/// }));
/// ```
pub struct FnSystem<F: Fn(&mut World, f64) + Send + Sync + 'static> {
    func: F,
    stage: SystemStage,
    name: &'static str,
}

impl<F: Fn(&mut World, f64) + Send + Sync + 'static> FnSystem<F> {
    pub fn new(func: F) -> Self {
        Self {
            func,
            stage: SystemStage::Update,
            name: "anonymous_fn_system",
        }
    }

    pub fn with_stage(mut self, stage: SystemStage) -> Self {
        self.stage = stage;
        self
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }
}

impl<F: Fn(&mut World, f64) + Send + Sync + 'static> System for FnSystem<F> {
    fn run(&mut self, world: &mut World, dt: f64) {
        (self.func)(world, dt);
    }

    fn stage(&self) -> SystemStage {
        self.stage
    }

    fn name(&self) -> &str {
        self.name
    }
}

/// Marker for system registration. Implemented by `FnSystem`, named structs, etc.
pub trait IntoSystem<Marker> {
    fn into_system(self) -> BoxedSystem;
}

impl<F: Fn(&mut World, f64) + Send + Sync + 'static> IntoSystem<()> for F {
    fn into_system(self) -> BoxedSystem {
        Box::new(FnSystem::new(self))
    }
}

/// Schedules and runs systems in stage order.
///
/// Phase 1: Simple sequential execution sorted by `SystemStage`.
/// Phase 2: Parallel dispatch via rayon DAG based on `WorldAccess` conflicts.
#[derive(Default)]
pub struct SystemScheduler {
    systems: Vec<(SystemStage, BoxedSystem)>,
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system<S: System>(&mut self, system: S) {
        let stage = system.stage();
        self.systems.push((stage, Box::new(system)));
        // Keep sorted by stage for in-order execution
        self.systems.sort_by_key(|(s, _)| *s);
    }

    pub fn add_system_to_stage<S: System>(&mut self, stage: SystemStage, mut system: S) {
        self.systems.push((stage, Box::new(system)));
        self.systems.sort_by_key(|(s, _)| *s);
    }

    /// Run all systems for a given stage.
    pub fn run_stage(&mut self, stage: SystemStage, world: &mut World, dt: f64) {
        for (s, system) in self.systems.iter_mut() {
            if *s == stage {
                let _span = tracing::debug_span!("system", name = system.name()).entered();
                system.run(world, dt);
            }
        }
    }

    /// Run all stages in order.
    pub fn run_all(&mut self, world: &mut World, dt: f64) {
        for stage in [
            SystemStage::PreUpdate,
            SystemStage::Update,
            SystemStage::PostUpdate,
            SystemStage::Render,
            SystemStage::PostRender,
        ] {
            self.run_stage(stage, world, dt);
        }
    }
}
