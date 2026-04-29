use crate::components::{ColliderDesc, ColliderShape, PhysicsHandles, RigidBodyDesc, RigidBodyType};
use crate::world::PhysicsWorld;
use canopy_ecs::world::World;
use canopy_renderer::Transform;
use rapier3d::prelude::*;

pub fn physics_sync_system(world: &mut World, dt: f64) {
    // 1. Gather entities that need initialization
    let entities_to_init = world.query_filtered(&[
        canopy_ecs::component::ComponentId::of::<RigidBodyDesc>(),
        canopy_ecs::component::ComponentId::of::<Transform>(),
    ]);

    let mut init_data = Vec::new();
    for entity in entities_to_init {
        if world.has::<PhysicsHandles>(entity) {
            continue;
        }

        let rb_desc = world.get::<RigidBodyDesc>(entity).unwrap().clone();
        let transform = world.get::<Transform>(entity).unwrap().clone();
        let collider_desc = world.get::<ColliderDesc>(entity).cloned();
        init_data.push((entity, rb_desc, transform, collider_desc));
    }

    let mut to_add_handles = Vec::new();

    // 2. Initialize in physics world
    if let Some(physics_world) = world.get_resource_mut::<PhysicsWorld>() {
        for (entity, rb_desc, transform, collider_desc) in init_data {
            let rb = RigidBodyBuilder::new(match rb_desc.body_type {
                RigidBodyType::Dynamic => rapier3d::prelude::RigidBodyType::Dynamic,
                RigidBodyType::Fixed => rapier3d::prelude::RigidBodyType::Fixed,
            })
            .position(rapier3d::na::Isometry3::from_parts(
                rapier3d::na::Translation3::new(transform.position.x, transform.position.y, transform.position.z),
                rapier3d::na::UnitQuaternion::from_quaternion(
                    rapier3d::na::Quaternion::new(
                        transform.rotation.w,
                        transform.rotation.x,
                        transform.rotation.y,
                        transform.rotation.z,
                    )
                ),
            ))
            .build();

            let body_handle = physics_world.rigid_body_set.insert(rb);

            let collider_handle = if let Some(c_desc) = collider_desc {
                let shape = match c_desc.shape {
                    ColliderShape::Cuboid { half_extents } => {
                        SharedShape::cuboid(half_extents.x, half_extents.y, half_extents.z)
                    }
                };
                let collider = ColliderBuilder::new(shape).build();
                Some(physics_world.collider_set.insert_with_parent(
                    collider,
                    body_handle,
                    &mut physics_world.rigid_body_set,
                ))
            } else {
                None
            };

            to_add_handles.push((entity, PhysicsHandles { body_handle, collider_handle }));
        }

        // Step simulation
        physics_world.step(dt as f32);
    }

    // Insert handles into ECS
    for (entity, handles) in to_add_handles {
        world.insert(entity, handles);
    }

    // 3. Gather handles for syncing back
    let entities_to_sync = world.query_filtered(&[
        canopy_ecs::component::ComponentId::of::<PhysicsHandles>(),
        canopy_ecs::component::ComponentId::of::<Transform>(),
    ]);

    let mut sync_data = Vec::new();
    for entity in entities_to_sync {
        let handles = world.get::<PhysicsHandles>(entity).unwrap().clone();
        sync_data.push((entity, handles));
    }

    let mut updates = Vec::new();
    if let Some(physics_world) = world.get_resource_mut::<PhysicsWorld>() {
        for (entity, handles) in sync_data {
            if let Some(body) = physics_world.rigid_body_set.get(handles.body_handle) {
                if body.is_dynamic() {
                    let pos = body.translation();
                    let rot = body.rotation();
                    updates.push((
                        entity,
                        glam::Vec3::new(pos.x, pos.y, pos.z),
                        glam::Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w),
                    ));
                }
            }
        }
    }

    // Apply updates to Transform
    for (entity, pos, rot) in updates {
        if let Some(t) = world.get_mut::<Transform>(entity) {
            t.position = pos;
            t.rotation = rot;
        }
    }
}
