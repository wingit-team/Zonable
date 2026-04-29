import os
import canopy
from canopy import CanopyApp, EngineConfig, Vec3, Quat, Input, on_tick, on_init
from canopy.components import Transform, Mesh, RigidBody, Collider

# --- Cube Management ---

@on_init
def setup():
    print("Initializing Ground Entity...")
    ground = canopy.world.spawn()
    canopy.world.add(ground, Transform(position=Vec3(0, -2, 0), scale=Vec3(10, 0.1, 10)))
    canopy.world.add(ground, Mesh(asset="cube.canasset"))
    canopy.world.add(ground, RigidBody(body_type="fixed"))
    canopy.world.add(ground, Collider(shape="cuboid", half_extents=Vec3(5.0, 0.05, 5.0)))

    print("Initializing Cube Entity...")
    cube = canopy.world.spawn()
    canopy.world.add(cube, Transform(position=Vec3(0, 5, 0)))
    canopy.world.add(cube, Mesh(asset="cube.canasset"))
    canopy.world.add(cube, RigidBody(body_type="dynamic"))
    canopy.world.add(cube, Collider(shape="cuboid", half_extents=Vec3(0.5, 0.5, 0.5)))
    canopy.cube_entity = cube

@on_tick
def rotate_cube(dt, query):
    
    for entity, (transform,) in query.with_components(Transform):
        # Move based on W/S/A/D keys
        move_speed = 5.0
        if Input.is_key_held("W"):
            transform.position += Vec3(0, 0, -move_speed * dt)
        if Input.is_key_held("S"):
            transform.position += Vec3(0, 0, move_speed * dt)
        if Input.is_key_held("A"):
            transform.position += Vec3(-move_speed * dt, 0, 0)
        if Input.is_key_held("D"):
            transform.position += Vec3(move_speed * dt, 0, 0)
        
        # Rotate based on Q and E keys
        rot_speed = 2.0
        if Input.is_key_held("Q"):
            transform.rotation *= Quat.from_axis_angle(Vec3(0, 1, 0), rot_speed * dt)
        if Input.is_key_held("E"):
            transform.rotation *= Quat.from_axis_angle(Vec3(0, 1, 0), -rot_speed * dt)

# --- Main Entry Point ---

def main():
    # Resolve absolute paths relative to this file
    base_dir = os.path.dirname(os.path.abspath(__file__))
    assets_dir = os.path.join(base_dir, "assets")
    scripts_dir = os.path.join(base_dir, "scripts")

    config = EngineConfig(
        title="Zonable — City Builder",
        width=1280,
        height=720,
        assets_dir=assets_dir,
        scripts_dir=scripts_dir,
    )

    app = CanopyApp(config)
    print("Launching Canopy Engine...")
    app.run()

if __name__ == "__main__":
    main()
