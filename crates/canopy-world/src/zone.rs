//! Zone map — sparse grid of zone types.

use ahash::AHashMap;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneType {
    Empty,
    Residential,
    Commercial,
    Industrial,
    Office,
    Park,
    Civic,
    Road,
}

#[derive(Debug, Clone)]
pub struct ZoneCell {
    pub zone_type: ZoneType,
    pub tier: u8,        // 1-3 density tier
    pub damage: f32,     // 0.0 = intact, 1.0 = destroyed
    pub zone_id: u32,    // Unique zone identifier for economics
}

/// Sparse grid of zone cells. Empty cells are not stored.
pub struct ZoneMap {
    pub cells: AHashMap<(i32, i32), ZoneCell>,
    pub cell_size_meters: f32,
    next_zone_id: u32,
}

impl ZoneMap {
    pub fn new(cell_size: f32) -> Self {
        Self { cells: AHashMap::new(), cell_size_meters: cell_size, next_zone_id: 1 }
    }

    pub fn world_to_cell(&self, pos: Vec3) -> (i32, i32) {
        (
            (pos.x / self.cell_size_meters).floor() as i32,
            (pos.z / self.cell_size_meters).floor() as i32,
        )
    }

    pub fn set_zone(&mut self, pos: Vec3, zone_type: ZoneType, tier: u8) -> u32 {
        let coord = self.world_to_cell(pos);
        let zone_id = self.next_zone_id;
        self.next_zone_id += 1;
        self.cells.insert(coord, ZoneCell { zone_type, tier, damage: 0.0, zone_id });
        zone_id
    }

    pub fn get_zone(&self, pos: Vec3) -> Option<&ZoneCell> {
        self.cells.get(&self.world_to_cell(pos))
    }

    /// Query all zones within a radius. Returns zone_ids.
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u32> {
        let r_cells = (radius / self.cell_size_meters).ceil() as i32;
        let (cx, cz) = self.world_to_cell(center);
        let mut result = Vec::new();
        for x in (cx - r_cells)..=(cx + r_cells) {
            for z in (cz - r_cells)..=(cz + r_cells) {
                let dx = (x - cx) as f32 * self.cell_size_meters;
                let dz = (z - cz) as f32 * self.cell_size_meters;
                if (dx * dx + dz * dz).sqrt() <= radius {
                    if let Some(cell) = self.cells.get(&(x, z)) {
                        result.push(cell.zone_id);
                    }
                }
            }
        }
        result
    }

    pub fn apply_damage(&mut self, zone_id: u32, severity: f32) {
        for cell in self.cells.values_mut() {
            if cell.zone_id == zone_id {
                cell.damage = (cell.damage + severity).min(1.0);
            }
        }
    }
}
