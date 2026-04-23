"""Disaster event handlers.

Each handler is registered with @on_event and receives the event object
when it is published by the simulation layer or by player-triggered events.

# Disaster Severity Scale

Canopy uses a normalized severity [0.0, 1.0] regardless of disaster type:
- < 0.2: Minor event — cosmetic damage, no casualties
- 0.2-0.5: Moderate — structural damage, some displacement
- 0.5-0.8: Major — widespread damage, emergency services overwhelmed
- > 0.8: Catastrophic — city-wide crisis, rebuilding phase

# Cascading Effects

Disasters should trigger cascading simulation effects:
- Earthquake → building damage → residents displaced → housing demand spike
- Earthquake → road damage → traffic gridlock → economic slowdown
- Earthquake + power outage → hospitals overwhelmed → mortality spike

These cascades are modeled through event chaining (one @on_event handler
publishes follow-up events).
"""

from __future__ import annotations

from canopy import on_event
from canopy.sim import EarthquakeEvent, DisasterEvent


@on_event(EarthquakeEvent)
def handle_earthquake(event: EarthquakeEvent) -> None:
    """Process an earthquake event.

    Applies damage to all zones within the affected radius, triggers
    evacuation for severe quakes, and cascades into a power outage
    if infrastructure buildings are destroyed.

    Args:
        event: EarthquakeEvent with epicenter, magnitude, and depth.
    """
    # Import here to avoid circular imports at module load time
    from canopy.world import zone_map
    from canopy.sim import SimEvent, event_bus

    severity = event.severity  # [0.0, 1.0] derived from magnitude
    radius = event.affected_radius_meters

    print(f"[DISASTER] Earthquake M{event.magnitude:.1f} at {event.epicenter}, "
          f"radius {radius/1000:.1f}km, severity {severity:.2f}")

    # Apply structural damage to all zones in radius
    affected_zones = zone_map.query_radius(event.epicenter, radius)
    for zone_id in affected_zones:
        # Damage falls off with distance from epicenter
        zone_center = zone_map.get_zone_center(zone_id)
        dist = event.epicenter.distance(zone_center)
        dist_factor = max(0.0, 1.0 - dist / radius)
        zone_damage = severity * dist_factor * dist_factor  # Quadratic falloff

        zone_map.apply_damage(zone_id, zone_damage)

    # Trigger evacuation for severe quakes (magnitude > 6.5)
    if event.magnitude > 6.5:
        for zone_id in affected_zones:
            zone_cell = zone_map.get_zone_by_id(zone_id)
            if zone_cell and zone_cell.damage > 0.5:
                print(f"  [EVACUATION] Zone {zone_id} — damage {zone_cell.damage:.0%}")
                # Phase 2: trigger evacuation AI behavior in sim world

    # Cascade: infrastructure damage → power outage
    if severity > 0.6:
        outage_zones = [z for z in affected_zones
                        if _get_zone_damage(zone_map, z) > 0.4]
        if outage_zones:
            print(f"  [CASCADE] Power outage triggered for {len(outage_zones)} zones")
            event_bus.publish(SimEvent.power_outage(
                zone_ids=outage_zones,
                duration_ticks=int(severity * 200),  # Longer outage for higher severity
            ))


@on_event(DisasterEvent)
def log_disaster_to_ui(event: DisasterEvent) -> None:
    """Log all disaster events to the game UI notification system.

    This handler runs for all disaster types. It pushes a notification
    to the UI layer which Phase 2 will wire to the React HUD overlay.
    """
    # Phase 2: push to UI event queue → display notification banner
    print(f"[UI] Disaster event: {type(event).__name__} severity={event.severity:.2f}")


def _get_zone_damage(zone_map, zone_id: int) -> float:
    cell = zone_map.get_zone_by_id(zone_id)
    return cell.damage if cell else 0.0
