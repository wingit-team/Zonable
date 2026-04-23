"""Population growth simulation system.

Runs at heartbeat rate (4Hz) — far below frame rate. Manages the flow of
citizens from the statistical pool into buildings (occupancy) and vice versa.

# Design

Population growth in Zonable is demand-driven:
- Each zone generates a housing demand based on economic conditions
- Citizens move in when demand > 0 AND buildings have spare capacity
- Citizens leave (despawn/emigrate) when happiness falls below threshold

The `StatPool` represents citizens not individually simulated — they exist
as aggregate statistics per zone. When a citizen enters the active simulation
radius, they are "materialized" into a full `Agent` by the `TickBudgetManager`.

# Economic coupling

Population changes feed back into the economy:
- More residents → more local retail demand (commercial zones benefit)
- More jobs → attract more residents (industrial/office drives residential)
- High unemployment → population decline
- High taxes → happiness penalty → emigration

This creates the push-pull dynamics of a living city.
"""

from __future__ import annotations

from canopy import System, Query
from canopy.components import BuildingData, Zone


class PopulationGrowthSystem(System):
    """Manages citizen inflow/outflow between statistical pools and buildings.

    tick_rate = 4 means this system runs 4 times per second, not every frame.
    Long-term simulation (tax collection, GDP update) runs at 1Hz or once per day.
    """

    tick_rate = 4  # Hz

    def __init__(self):
        super().__init__()
        # Per-zone happiness cache — updated each tick from StatPool
        self._zone_happiness: dict[int, float] = {}
        # Track emigration pressure (sustained unhappiness = leave)
        self._emigration_pressure: dict[int, float] = {}

    def on_tick(self, dt: float, query: Query) -> None:
        """Process population changes for all residential buildings.

        For each building with spare capacity:
        1. Check StatPool housing demand for its zone
        2. Move citizens from pool into building occupancy
        3. Check happiness threshold — trigger emigration if unhappy
        """
        # TODO: Once StatPool is implemented in canopy-script bindings,
        # replace the stub calls below with real stat pool queries.

        for entity, (building, zone) in query.with_components(BuildingData, Zone):
            if zone.zone_type != "residential":
                continue

            if building.is_destroyed():
                continue  # Don't process destroyed buildings

            # --- Housing demand (stub: use zone_id as seed for demo) ---
            # Real implementation: pool = StatPool.for_zone(zone.zone_id)
            #                      demand = pool.housing_demand()
            demand = max(0, 10 - building.occupancy // 5)  # Placeholder

            # --- Fill vacancies ---
            if building.occupancy < building.capacity and demand > 0:
                # Move citizens in — rate limited to avoid sudden jumps
                max_incoming_per_tick = max(1, building.capacity // 20)
                incoming = min(demand, building.capacity - building.occupancy, max_incoming_per_tick)
                building.occupancy += incoming

            # --- Emigration pressure ---
            zone_happiness = self._zone_happiness.get(zone.zone_id, 0.7)
            if zone_happiness < 0.4:
                pressure = self._emigration_pressure.get(zone.zone_id, 0.0)
                pressure = min(pressure + dt * 0.1, 1.0)
                self._emigration_pressure[zone.zone_id] = pressure

                if pressure > 0.8 and building.occupancy > 0:
                    # Sustained unhappiness — some citizens leave
                    leaving = max(1, building.occupancy // 10)
                    building.occupancy = max(0, building.occupancy - leaving)
            else:
                # Reset pressure when conditions improve
                self._emigration_pressure[zone.zone_id] = max(
                    0.0,
                    self._emigration_pressure.get(zone.zone_id, 0.0) - dt * 0.2
                )

    def update_zone_happiness(self, zone_id: int, happiness: float) -> None:
        """Called by the economic system to report zone-level happiness."""
        self._zone_happiness[zone_id] = happiness


class ConstructionProgressSystem(System):
    """Advances construction progress for incomplete buildings.

    Buildings start at construction_progress=0.0 and are fully built at 1.0.
    The rate depends on available workers and materials (stub in Phase 1).
    """

    tick_rate = 4  # Hz

    def on_tick(self, dt: float, query: Query) -> None:
        for entity, (building,) in query.with_components(BuildingData):
            if building.is_complete():
                continue
            # Flat construction rate for now
            # Phase 2: rate = f(available_workers, materials, zone_tier)
            rate = 0.05 * dt  # ~20 ticks to complete = ~5 sim-seconds
            building.construction_progress = min(1.0, building.construction_progress + rate)
