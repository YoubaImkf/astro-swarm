use crate::communication::channels::ResourceType;
use crate::robot::core::knowledge::{RobotKnowledge, TileInfo};
use chrono::{DateTime, Utc};
use log::{debug, trace, warn};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ResourceVersion {
    pub amount: u32,
    pub timestamp: DateTime<Utc>,
    pub robot_id: u32,
    pub resource_type: ResourceType,
}

#[derive(Clone, Debug)]
pub enum GlobalTileInfo {
    Unknown,
    Walkable(DateTime<Utc>),
    Obstacle(DateTime<Utc>),
    Resource(ResourceVersion),
    Station,
}

pub struct DataManager {
    global_knowledge: HashMap<(usize, usize), GlobalTileInfo>,
    map_width: usize,
    map_height: usize,
}

impl DataManager {
    pub fn new(width: usize, height: usize) -> Self {
        let capacity = width * height;
        let mut global_knowledge = HashMap::with_capacity(capacity);
        let station_x = width / 2;
        let station_y = height / 2;

        for y in 0..height {
            for x in 0..width {
                let is_station_area = (x.abs_diff(station_x) <= 1) && (y.abs_diff(station_y) <= 1);

                if is_station_area {
                    global_knowledge.insert((x, y), GlobalTileInfo::Station);
                } else {
                    global_knowledge.insert((x, y), GlobalTileInfo::Unknown);
                }
            }
        }

        debug!(
            "DataManager initialized with dimensions {}x{}",
            width, height
        );
        Self {
            global_knowledge,
            map_width: width,
            map_height: height,
        }
    }

    /// My Logic : Merges knowledge reported by a specific robot into the global knowledge base
    /// Uses timestamps to resolve conflicts, prioritizing newer information
    pub fn merge_robot_knowledge(&mut self, robot_id: u32, knowledge: &RobotKnowledge) {
        let now = Utc::now();
        trace!("Merging knowledge from Robot {}", robot_id);
        for (&(x, y), robot_tile_info) in &knowledge.map {
            if x >= self.map_width || y >= self.map_height {
                warn!(
                    "Robot {} reported knowledge for out-of-bounds tile ({}, {}). Skipping.",
                    robot_id, x, y
                );
                continue;
            }

            if let Some(GlobalTileInfo::Station) = self.global_knowledge.get(&(x, y)) {
                if !matches!(robot_tile_info, TileInfo::Station) {
                    trace!(
                        "Skipping update for tile ({},{}) as it's part of the station.",
                        x,
                        y
                    );
                    continue;
                }
            }

            // Convert robot's TileInfo to a potential GlobalTileInfo update
            let potential_update = match robot_tile_info {
                TileInfo::Unknown => None,
                TileInfo::Walkable => Some(GlobalTileInfo::Walkable(now)),
                TileInfo::Obstacle => Some(GlobalTileInfo::Obstacle(now)),
                TileInfo::Resource(res_type, amount) => {
                    let version = ResourceVersion {
                        amount: *amount,
                        timestamp: now,
                        robot_id,
                        resource_type: res_type.clone(),
                    };
                    Some(GlobalTileInfo::Resource(version))
                }
                TileInfo::Station => Some(GlobalTileInfo::Station),
            };

            if let Some(new_info) = potential_update {
                self.update_global_tile(x, y, new_info);
            }
        }
    }

    // Update global tile, resolving conflicts (latest timestamp wins)
    pub fn update_global_tile(&mut self, x: usize, y: usize, new_info: GlobalTileInfo) {
        match self.global_knowledge.entry((x, y)) {
            Entry::Occupied(mut occ) => {
                let current: &GlobalTileInfo = occ.get();
                let should_update = match (current, &new_info) {
                    (GlobalTileInfo::Station, _) => false,
                    (_, GlobalTileInfo::Station) => true,
                    (GlobalTileInfo::Unknown, _) => true,

                    (GlobalTileInfo::Walkable(cts), GlobalTileInfo::Walkable(nts)) => nts > cts,
                    (GlobalTileInfo::Obstacle(cts), GlobalTileInfo::Obstacle(nts)) => nts > cts,
                    (GlobalTileInfo::Resource(cv), GlobalTileInfo::Resource(nv)) => {
                        nv.timestamp > cv.timestamp
                    }

                    (GlobalTileInfo::Walkable(cts), GlobalTileInfo::Resource(nv)) => {
                        nv.timestamp > *cts
                    }
                    (GlobalTileInfo::Obstacle(cts), GlobalTileInfo::Resource(nv)) => {
                        nv.timestamp > *cts
                    }
                    (GlobalTileInfo::Resource(cv), GlobalTileInfo::Walkable(nts)) => {
                        *nts > cv.timestamp
                    }
                    (GlobalTileInfo::Resource(cv), GlobalTileInfo::Obstacle(nts)) => {
                        *nts > cv.timestamp
                    }

                    _ => false,
                };

                if should_update {
                    trace!(
                        "Updating tile ({},{}): {:?} -> {:?}",
                        x,
                        y,
                        current,
                        new_info
                    );
                    *occ.get_mut() = new_info;
                } else {
                    trace!("Keeping existing tile ({},{}): {:?}", x, y, current);
                }
            }

            Entry::Vacant(vac) => {
                trace!("Inserting new tile ({},{}): {:?}", x, y, new_info);
                vac.insert(new_info);
            }
        }
    }

    /// This is sent back to robots after they dock.
    pub fn get_global_robot_knowledge(&self) -> RobotKnowledge {
        let mut robot_knowledge = RobotKnowledge::new(self.map_width, self.map_height);
        for (&(x, y), global_info) in &self.global_knowledge {
            let tile_info = match global_info {
                GlobalTileInfo::Unknown => TileInfo::Unknown,
                GlobalTileInfo::Walkable(_) => TileInfo::Walkable,
                GlobalTileInfo::Obstacle(_) => TileInfo::Obstacle,
                GlobalTileInfo::Resource(version) => {
                    TileInfo::Resource(version.resource_type.clone(), version.amount)
                }
                GlobalTileInfo::Station => TileInfo::Station,
            };
            robot_knowledge.update_tile(x, y, tile_info);
        }
        robot_knowledge
    }

    // This should be called periodically by the App/Station
    pub fn update_simulation_map(&self, map: &mut crate::map::noise::Map) {
        for (&(x, y), global_info) in &self.global_knowledge {
            match global_info {
                GlobalTileInfo::Resource(version) => {
                    let map_resource = map.get_resource(x, y);
                    let map_resource_type = map_resource.as_ref().map(|(rt, _)| rt);

                    if map_resource.is_none() && version.amount > 0 {
                    } else if let Some((_, map_amount)) = map_resource.as_ref() {
                        if version.amount == 0
                            && *map_amount > 0
                            && map_resource_type == Some(&version.resource_type)
                        {
                            // Remove resource if fully consumed
                            if matches!(
                                version.resource_type,
                                ResourceType::Energy | ResourceType::Minerals
                            ) {
                                map.remove_resource(x, y);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
