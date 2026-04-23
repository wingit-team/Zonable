//! Supply/demand economic ledger.

use ahash::AHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Housing, Jobs, Goods, Power, Water, Food, Healthcare, Education,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneType {
    Residential, Commercial, Industrial, Office, Park, Civic,
}

/// Per-zone economic register.
#[derive(Debug, Clone, Default)]
pub struct ZoneEconomy {
    pub supply: AHashMap<ResourceType, f32>,
    pub demand: AHashMap<ResourceType, f32>,
    pub price: AHashMap<ResourceType, f32>,
}

impl ZoneEconomy {
    /// Update price based on supply/demand. Called each sim tick.
    /// Simple equilibrium model: price rises when demand > supply.
    pub fn tick_prices(&mut self) {
        for (resource, &demand) in &self.demand {
            let supply = self.supply.get(resource).copied().unwrap_or(0.0);
            let price = self.price.entry(*resource).or_insert(1.0);
            let ratio = if supply > 0.0 { demand / supply } else { 10.0 };
            // Damped price adjustment — converges toward equilibrium
            *price = (*price * 0.95 + ratio * 0.05).clamp(0.1, 100.0);
        }
    }
}

/// City-wide economic ledger aggregating all zones.
pub struct EconomyLedger {
    pub zones: AHashMap<u32, ZoneEconomy>, // zone_id → economy
    pub global_gdp: f64,
    pub unemployment_rate: f32,
    pub inflation: f32,
}

impl EconomyLedger {
    pub fn new() -> Self {
        Self {
            zones: AHashMap::new(),
            global_gdp: 0.0,
            unemployment_rate: 0.05,
            inflation: 0.02,
        }
    }

    pub fn zone_mut(&mut self, zone_id: u32) -> &mut ZoneEconomy {
        self.zones.entry(zone_id).or_default()
    }

    pub fn tick(&mut self) {
        for zone in self.zones.values_mut() {
            zone.tick_prices();
        }
        // TODO Phase 2: aggregate GDP, compute global unemployment, CPI
    }
}

impl Default for EconomyLedger {
    fn default() -> Self { Self::new() }
}
