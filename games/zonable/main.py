"""Zonable — city builder powered by Canopy Engine.

Entry point. Run this file to start the game:
    python main.py

This script:
1. Configures the engine
2. Launches CanopyApp (blocks until window closed)

All game logic lives in scripts/ — the engine imports those automatically.
"""

from canopy import CanopyApp, EngineConfig

config = EngineConfig(
    title="Zonable — City Builder",
    width=2560,
    height=1440,
    vsync=True,
    headless=False,
    assets_dir="assets",
    scripts_dir="scripts",
    target_tick_hz=60,
    heartbeat_hz=4,
    asset_memory_mb=4096,
)

if __name__ == "__main__":
    app = CanopyApp(config)
    app.run()
