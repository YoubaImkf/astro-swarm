use std::collections::HashMap;
use crate::communication::channels::ResourceType;

#[derive(Clone, Debug)]
pub enum TileInfo {
    Unknown,
    Walkable,
    Obstacle,
    Resource(ResourceType, u32),
    Station,
}

#[derive(Clone, Debug)]
pub struct RobotKnowledge {
    pub map: HashMap<(usize, usize), TileInfo>,
    pub width: usize,
    pub height: usize,
}

impl RobotKnowledge {
    pub fn new(width: usize, height: usize) -> Self {
        let mut map = HashMap::new();
        // Init all tiles are unknown
        for y in 0..height {
            for x in 0..width {
                map.insert((x, y), TileInfo::Unknown);
            }
        }
        Self { map, width, height }
    }

    pub fn update_tile(&mut self, x: usize, y: usize, info: TileInfo) {
        self.map.insert((x, y), info);
    }

    pub fn get_tile(&self, x: usize, y: usize) -> &TileInfo {
        self.map.get(&(x, y)).unwrap_or(&TileInfo::Unknown)
    }

    pub fn observe_and_update(&mut self, x: usize, y: usize, map: &crate::map::noise::Map) {
        if map.is_station(x, y) {
            self.update_tile(x, y, TileInfo::Station);
        } else if map.is_obstacle(x, y) {
            self.update_tile(x, y, TileInfo::Obstacle);
        } else if let Some((res_type, amount)) = map.get_resource(x, y) {
            self.update_tile(x, y, TileInfo::Resource(res_type, amount));
        } else {
            self.update_tile(x, y, TileInfo::Walkable);
        }
    }
}