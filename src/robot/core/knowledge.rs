use log::error;
use std::collections::HashMap;

use crate::communication::channels::ResourceType;
use crate::map::noise::Map;

#[derive(Clone, Debug, PartialEq)]
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
        let capacity = width * height;
        let mut map = HashMap::with_capacity(capacity);

        for y in 0..height {
            for x in 0..width {
                map.insert((x, y), TileInfo::Unknown);
            }
        }

        let center_x = width / 2;
        let center_y = height / 2;
        map.insert((center_x, center_y), TileInfo::Station);
        Self { map, width, height }
    }

    pub fn update_tile(&mut self, x: usize, y: usize, info: TileInfo) {
        if x < self.width && y < self.height {
            self.map.insert((x, y), info);
        } else {
            error!(
                "Attempted to update knowledge out of bounds at ({}, {})",
                x, y
            );
        }
    }

    pub fn get_tile(&self, x: usize, y: usize) -> &TileInfo {
        self.map.get(&(x, y)).unwrap_or(&TileInfo::Unknown)
    }

    pub fn observe_and_update(&mut self, x: usize, y: usize, map: &Map) {
        if x >= map.width || y >= map.height {
            error!("Attempted to observe map out of bounds at ({}, {})", x, y);
            return;
        }

        let info = if map.is_station(x, y) {
            TileInfo::Station
        } else if map.is_obstacle(x, y) {
            TileInfo::Obstacle
        } else if let Some((res_type, amount)) = map.get_resource(x, y) {
            if amount > 0 {
                TileInfo::Resource(res_type, amount)
            } else {
                TileInfo::Walkable
            }
        } else {
            TileInfo::Walkable
        };
        self.update_tile(x, y, info);
    }

    pub fn get_station_coords(&self) -> (usize, usize) {
        (self.width / 2, self.height / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_initializes_unknown_and_station() {
        let width = 10;
        let height = 10;
        let knowledge = RobotKnowledge::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let tile = knowledge.get_tile(x, y);
                if (x, y) == (width / 2, height / 2) {
                    assert_eq!(tile, &TileInfo::Station);
                } else {
                    assert_eq!(tile, &TileInfo::Unknown);
                }
            }
        }
    }

    #[test]
    fn test_update_and_get_tile() {
        let mut knowledge = RobotKnowledge::new(5, 5);
        knowledge.update_tile(1, 2, TileInfo::Walkable);
        assert_eq!(knowledge.get_tile(1, 2), &TileInfo::Walkable);
    }

    #[test]
    fn test_update_tile_out_of_bounds_does_not_panic() {
        let mut knowledge = RobotKnowledge::new(3, 3);
        knowledge.update_tile(10, 10, TileInfo::Obstacle);
        assert_eq!(knowledge.get_tile(10, 10), &TileInfo::Unknown);
    }

    #[test]
    fn test_get_station_coords() {
        let knowledge = RobotKnowledge::new(8, 6);
        assert_eq!(knowledge.get_station_coords(), (4, 3));
    }
}
