"""Citizen query utilities.

Provides functions for querying citizen data within zones, used by:
- The city stats panel
- AI advisor system
- Debug/inspector overlay
"""

from __future__ import annotations

from typing import Any


def get_citizens_in_zone(zone_id: int) -> dict[str, Any]:
    """Get a snapshot of citizen data for a zone.

    Combines:
    - Active agents (individually simulated, within camera radius)
    - Statistical population (aggregate, outside camera radius)

    Args:
        zone_id: The numeric zone identifier.

    Returns:
        Dict with counts, rates, and a sample of individual active agents.

    Note:
        `statistical_count` and rate metrics are computed from the StatPool
        and may lag by up to one heartbeat tick (250ms at 4Hz).
    """
    # Phase 2: import from canopy-script bindings
    # from canopy.sim import agent_world, StatPool

    # Phase 1 stub — returns plausible dummy data for UI development
    active_agents = _stub_get_active_agents(zone_id)
    stats = _stub_get_stat_pool(zone_id)

    return {
        "zone_id": zone_id,
        "active_count": len(active_agents),
        "statistical_count": stats["total"],
        "employed_ratio": stats["employment_rate"],
        "happiness": stats["happiness"],
        "housing_demand": stats["housing_demand"],
        "agents": [
            {
                "id": a["entity_id"],
                "job": a["job_type"],
                "home": a["home_zone"],
                "happiness": a["happiness"],
                "employment": a["employment"],
            }
            for a in active_agents[:100]  # Cap sample size
        ]
    }


def get_zone_economic_summary(zone_id: int) -> dict[str, Any]:
    """Get economic statistics for a zone.

    Used by the city stats panel to display per-zone economic health.
    """
    # Phase 2: query EconomyLedger from canopy-sim
    return {
        "zone_id": zone_id,
        "gdp_contribution": 0.0,    # Phase 2
        "tax_revenue": 0.0,          # Phase 2
        "unemployment_rate": 0.05,   # Phase 2
        "retail_demand": 0.0,        # Phase 2
        "housing_demand": 0.0,       # Phase 2
    }


def get_top_zones_by_population(n: int = 10) -> list[dict[str, Any]]:
    """Return the top N zones by total population (active + statistical).

    Used by the leaderboard overlay and AI advisor to identify
    high-density neighborhoods needing attention.
    """
    # Phase 2: iterate all zones from StatPool registry
    return []


# ---------------------------------------------------------------------------
# Stubs (replace with real canopy-sim bindings in Phase 2)
# ---------------------------------------------------------------------------

def _stub_get_active_agents(zone_id: int) -> list[dict]:
    """Stub: returns fake active agents for zone_id."""
    import random
    rng = random.Random(zone_id)
    n = rng.randint(0, 15)
    job_types = ["office_worker", "retail_worker", "factory_worker", "student", "retired"]
    return [
        {
            "entity_id": rng.randint(1000, 999999),
            "job_type": rng.choice(job_types),
            "home_zone": zone_id,
            "happiness": round(rng.uniform(0.3, 0.95), 2),
            "employment": rng.choice(["employed", "unemployed", "retired"]),
        }
        for _ in range(n)
    ]


def _stub_get_stat_pool(zone_id: int) -> dict:
    """Stub: returns fake StatPool data."""
    import random
    rng = random.Random(zone_id + 1337)
    return {
        "total": rng.randint(50, 5000),
        "employment_rate": round(rng.uniform(0.7, 0.98), 3),
        "happiness": round(rng.uniform(0.4, 0.9), 2),
        "housing_demand": rng.randint(0, 100),
    }
