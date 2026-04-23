//! `EntityCommandBuffer` — deferred structural world mutations.
//!
//! Systems must not structurally modify the World while iterating it
//! (spawn/despawn/add-component/remove-component). Instead they push commands
//! into an `EntityCommandBuffer` and the `SystemScheduler` flushes it after
//! each stage completes.
//!
//! The command buffer is a `Vec<Command>` — append-only during a stage, drained
//! at flush. It is designed for ergonomic use in systems:
//!
//! ```rust
//! fn my_system(world: &mut World, dt: f64) {
//!     let mut cmds = EntityCommandBuffer::new();
//!     for (entity, hp) in world.query::<Health>() {
//!         if hp.0 <= 0.0 {
//!             cmds.despawn(entity);
//!         }
//!     }
//!     cmds.flush(world);
//! }
//! ```

use crate::component::Component;
use crate::entity::Entity;
use crate::world::World;

/// A deferred structural world command.
enum Command {
    Spawn {
        /// Components will be applied to this pre-allocated entity ID.
        entity: Entity,
        appliers: Vec<Box<dyn ComponentApplier>>,
    },
    Despawn(Entity),
    InsertComponent {
        entity: Entity,
        applier: Box<dyn ComponentApplier>,
    },
    RemoveComponent {
        entity: Entity,
        remover: Box<dyn ComponentRemover>,
    },
}

/// Type-erased "insert this component onto entity" action.
trait ComponentApplier: Send + Sync {
    fn apply(self: Box<Self>, entity: Entity, world: &mut World);
}

struct ConcreteApplier<T: Component> {
    value: T,
}

impl<T: Component> ComponentApplier for ConcreteApplier<T> {
    fn apply(self: Box<Self>, entity: Entity, world: &mut World) {
        world.insert(entity, self.value);
    }
}

/// Type-erased "remove component T from entity" action.
trait ComponentRemover: Send + Sync {
    fn remove(self: Box<Self>, entity: Entity, world: &mut World);
}

struct ConcreteRemover<T: Component>(std::marker::PhantomData<T>);

impl<T: Component> ComponentRemover for ConcreteRemover<T> {
    fn remove(self: Box<Self>, entity: Entity, world: &mut World) {
        world.remove::<T>(entity);
    }
}

// ---------------------------------------------------------------------------

/// Deferred entity command buffer. Collect during system run, flush after stage.
pub struct EntityCommandBuffer {
    commands: Vec<Command>,
}

impl EntityCommandBuffer {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Queue a new entity spawn with the given component bundle.
    /// Returns a *reserved* `Entity` ID that will be valid after `flush`.
    ///
    /// Note: The entity is allocated immediately (so you can reference it in
    /// subsequent commands), but components are not inserted until flush.
    pub fn spawn(&mut self, world: &mut World) -> EntityBuilder<'_> {
        // We pre-allocate the entity now so callers can reference it
        let entity = world.spawn();
        let idx = self.commands.len();
        self.commands.push(Command::Spawn {
            entity,
            appliers: Vec::new(),
        });
        EntityBuilder { buffer: self, idx, entity }
    }

    /// Queue despawning an entity.
    pub fn despawn(&mut self, entity: Entity) {
        self.commands.push(Command::Despawn(entity));
    }

    /// Queue inserting a component onto an existing entity.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.commands.push(Command::InsertComponent {
            entity,
            applier: Box::new(ConcreteApplier { value }),
        });
    }

    /// Queue removing a component from an entity.
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.commands.push(Command::RemoveComponent {
            entity,
            remover: Box::new(ConcreteRemover::<T>(Default::default())),
        });
    }

    /// Apply all queued commands to the world. Called by the `SystemScheduler`
    /// between stages (and can be called manually at end of a system).
    pub fn flush(self, world: &mut World) {
        for command in self.commands {
            match command {
                Command::Spawn { entity, appliers } => {
                    // Entity was already allocated in `spawn()`, just apply components
                    for applier in appliers {
                        applier.apply(entity, world);
                    }
                }
                Command::Despawn(entity) => {
                    world.despawn(entity);
                }
                Command::InsertComponent { entity, applier } => {
                    if world.is_alive(entity) {
                        applier.apply(entity, world);
                    }
                }
                Command::RemoveComponent { entity, remover } => {
                    if world.is_alive(entity) {
                        remover.remove(entity, world);
                    }
                }
            }
        }
    }
}

impl Default for EntityCommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder returned by `EntityCommandBuffer::spawn`. Chain `.with(component)` calls.
pub struct EntityBuilder<'a> {
    buffer: &'a mut EntityCommandBuffer,
    idx: usize,
    entity: Entity,
}

impl<'a> EntityBuilder<'a> {
    /// Queue adding a component to the entity being spawned.
    pub fn with<T: Component>(self, value: T) -> Self {
        if let Command::Spawn { ref mut appliers, .. } = self.buffer.commands[self.idx] {
            appliers.push(Box::new(ConcreteApplier { value }));
        }
        self
    }

    /// Finish building and return the pre-allocated entity ID.
    pub fn id(self) -> Entity {
        self.entity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Marker;

    #[test]
    fn deferred_spawn_and_despawn() {
        let mut world = World::new();
        let mut cmds = EntityCommandBuffer::new();

        let e = cmds.spawn(&mut world).id();
        assert!(world.is_alive(e)); // pre-allocated
        cmds.despawn(e);
        cmds.flush(&mut world);
        assert!(!world.is_alive(e));
    }
}
