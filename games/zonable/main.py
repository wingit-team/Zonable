import os
import canopy
from canopy import CanopyApp, EngineConfig, Vec3, Quat, Input, on_tick, on_init
from canopy.components import Transform, Mesh

# --- Cube Management ---

@on_init
def setup():
    print("Initializing Cube Entity...")
    cube = canopy.world.spawn()
    canopy.world.add(cube, Transform(position=Vec3(0, 0, 0)))
    canopy.world.add(cube, Mesh(asset="cube.canasset"))
    canopy.cube_entity = cube

@on_tick
def rotate_cube(dt, query):
    
    for entity, (transform,) in query.with_components(Transform):
        # Rotate based on A and D keys
        speed = 2.0
        if Input.is_key_held("A"):
            transform.rotation *= Quat.from_axis_angle(Vec3(0, 1, 0), speed * dt)
        if Input.is_key_held("D"):
            transform.rotation *= Quat.from_axis_angle(Vec3(0, 1, 0), -speed * dt)

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
