use log::{debug, error, info, warn};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use super::knowledge::{self, RobotKnowledge, TileInfo};
use super::{common, config, movement, RobotState};
use crate::communication::channels::{ResourceType, RobotEvent};
use crate::map::noise::Map;
use crate::robot::movement::Direction;
use crate::robot::state::RobotStatus;

const RANDOM_MOVE_ATTEMPTS: usize = 4;

pub struct CollectionRobot {
    state: RobotState,
    target_resource_type: Option<ResourceType>,
    knowledge: RobotKnowledge,
    merge_complete_receiver: Receiver<RobotEvent>,
    current_target_coords: Option<(usize, usize)>,
    config: config::RobotTypeConfig,
}

impl CollectionRobot {
    pub fn new(
        initial_state: RobotState,
        map_width: usize,
        map_height: usize,
        merge_complete_receiver: Receiver<RobotEvent>,
    ) -> Self {
        Self {
            knowledge: RobotKnowledge::new(map_width, map_height),
            state: initial_state,
            target_resource_type: Some(ResourceType::Minerals),
            merge_complete_receiver,
            current_target_coords: None,
            config: config::COLLECTION_CONFIG.clone(),
        }
    }

    pub fn set_target_resource(&mut self, resource_type: ResourceType) {
        if matches!(resource_type, ResourceType::Energy | ResourceType::Minerals) {
            info!(
                "Robot {}: Setting target resource type to {:?}",
                self.state.id, resource_type
            );
            self.target_resource_type = Some(resource_type);
        } else {
            warn!(
                "Robot {}: Attempted to set invalid target resource type: {:?}",
                self.state.id, resource_type
            );
        }
    }

    fn find_nearest_target_resource(&self) -> Option<(usize, usize)> {
        let target_type = self.target_resource_type.as_ref()?;

        let known_resource = self
            .knowledge
            .map
            .iter()
            .filter_map(|(&(x, y), tile_info)| {
                if let TileInfo::Resource(res_type, amount) = tile_info {
                    if res_type == target_type && *amount > 0 {
                        Some((
                            (x, y),
                            (x as isize - self.state.x as isize).pow(2)
                                + (y as isize - self.state.y as isize).pow(2),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .min_by_key(|&(_, dist)| dist)
            .map(|(coords, _)| coords);

        if let Some(coords) = known_resource {
            debug!(
                "Robot: {} Found known target resource at {:?}",
                self.state.id, coords
            );
            return Some(coords);
        }

        let unknown_tile = self
            .knowledge
            .map
            .iter()
            .filter_map(|(&(x, y), tile_info)| {
                if matches!(tile_info, TileInfo::Unknown) {
                    Some((
                        (x, y),
                        (x as isize - self.state.x as isize).pow(2)
                            + (y as isize - self.state.y as isize).pow(2),
                    ))
                } else {
                    None
                }
            })
            .min_by_key(|&(_, dist)| dist)
            .map(|(coords, _)| coords);

        if let Some(coords) = unknown_tile {
            debug!(
                "Robot: {} No known target resource, found unknown tile at {:?}",
                self.state.id, coords
            );
            Some(coords)
        } else {
            debug!(
                "Robot: {} No known target resource or unknown tiles found.",
                self.state.id
            );
            None
        }
    }

    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        let robot_id = self.state.id;
        let station_coords = self.knowledge.get_station_coords();
        let config = self.config.clone();
        let collection_action_cost = config
            .action_energy_cost
            .expect("Collection config must have action cost");

        thread::spawn(move || {
            debug!(
                "Robot: {} Carrying {}/{} units",
                self.state.id,
                self.state.collected_resources.values().sum::<u32>(),
                self.state.max_capacity
            );
            info!(
                "Robot {}: Starting collection thread with capacity {}",
                robot_id, self.state.max_capacity
            );

            loop {
                match self.state.status {
                    RobotStatus::Collecting => {
                        self.handle_collecting(&sender, &map, collection_action_cost, &config);
                    }
                    RobotStatus::ReturningToStation => {
                        self.handle_returning_to_station(&sender, &map, station_coords, &config);
                    }
                    RobotStatus::AtStation => {
                        self.handle_at_station();
                    }
                    _ => {
                        error!("Robot: {} Unhandled state {:?}.", robot_id, self.state.status);
                        self.state.status = RobotStatus::Collecting;
                        thread::sleep(config::UNHANDLED_STATE_SLEEP);
                    }
                }
            }
        });
    }

    fn handle_collecting(
        &mut self,
        sender: &Sender<RobotEvent>,
        map: &Arc<RwLock<Map>>,
        collection_action_cost: u32,
        config: &config::RobotTypeConfig,
    ) {
        let robot_id = self.state.id;

        if self.state.energy <= config.low_energy_threshold || self.state.is_full() {
            info!(
                "Robot: {} {}",
                robot_id,
                if self.state.energy <= config.low_energy_threshold {
                    "Low energy, returning"
                } else {
                    "Full, returning"
                }
            );
            self.state.status = RobotStatus::ReturningToStation;
            self.current_target_coords = None;
            return;
        }

        let (current_x, current_y) = (self.state.x, self.state.y);

        let target_type = self.target_resource_type.clone();
        if let Some(target_type) = target_type {
            if self.try_collect_resource(
                current_x,
                current_y,
                &target_type,
                collection_action_cost,
                map,
                sender,
            ) {
                thread::sleep(config::random_sleep_duration(
                    config.primary_action_sleep_min_ms,
                    config.primary_action_sleep_max_ms,
                ));
                return;
            }
        }

        self.update_knowledge_around(map);

        let direction = if let Some(target_coords) = self.find_nearest_target_resource() {
            debug!(
                "Robot: {} Moving towards {:?} @ {:?} from {:?}",
                robot_id,
                self.target_resource_type.as_ref().unwrap(),
                target_coords,
                (self.state.x, self.state.y)
            );
            self.current_target_coords = Some(target_coords);

            common::move_towards_target(
                self.state.x,
                self.state.y,
                target_coords.0,
                target_coords.1,
                &self.knowledge,
                &*map.read().unwrap(),
            )
        } else {
            debug!(
                "Robot: {} No target {:?}. Enhanced exploring.",
                robot_id, self.target_resource_type
            );
            self.current_target_coords = None;
            self.choose_best_explore_direction(&*map.read().unwrap())
        };

        self.try_move(direction, map, config, sender);

        thread::sleep(config::random_sleep_duration(
            config.primary_action_sleep_min_ms,
            config.primary_action_sleep_max_ms,
        ));
    }

    fn try_collect_resource(
        &mut self,
        x: usize,
        y: usize,
        target_type: &ResourceType,
        collection_action_cost: u32,
        map: &Arc<RwLock<Map>>,
        sender: &Sender<RobotEvent>,
    ) -> bool {
        let robot_id = self.state.id;
        let resource_present = {
            let guard = match map.read() {
                Ok(g) => g,
                Err(p) => {
                    error!("Robot: {} Map read poisoned! {}", robot_id, p);
                    return false;
                }
            };
            guard
                .get_resource(x, y)
                .map_or(false, |(rt, amount)| rt == *target_type && amount > 0)
        };

        if !resource_present {
            debug!("Robot: {} No resource present at ({}, {})", robot_id, x, y);
            return false;
        }

        if !self.state.use_energy(collection_action_cost) {
            warn!(
                "Robot: {} No energy ({}) to collect @ {:?}",
                robot_id,
                self.state.energy,
                (x, y)
            );
            return false;
        }

        let mut amount_collected = 0;
        let mut remove_successful = false;
        {
            let mut guard = match map.write() {
                Ok(g) => g,
                Err(p) => {
                    error!("Robot: {} Map write poisoned! {}", robot_id, p);
                    return false;
                }
            };
            if let Some((res_type, amount)) = guard.get_resource(x, y) {
                debug!(
                    "Robot: {} Resource at ({}, {}): {:?} amount={}",
                    robot_id, x, y, res_type, amount
                );
                let current_total = self.state.collected_resources.values().sum::<u32>();
                let available_capacity = self.state.max_capacity.saturating_sub(current_total);
                let to_collect = amount.min(available_capacity);

                debug!(
                    "Robot: {} Carrying {}/{} before collecting. Trying to collect {}.",
                    robot_id, current_total, self.state.max_capacity, to_collect
                );

                if res_type == *target_type && amount > 0 {
                    if self.state.collect_resource(target_type.clone(), amount) {
                        amount_collected = amount;
                        if guard.remove_resource(x, y).is_some() {
                            remove_successful = true;
                            info!(
                                "Robot: {} Collected/removed {} {:?} @ {:?}. Now carrying {}/{}.",
                                robot_id,
                                amount_collected,
                                target_type,
                                (x, y),
                                self.state.collected_resources.values().sum::<u32>(),
                                self.state.max_capacity
                            );
                        } else {
                            error!("Robot: {} Failed remove map @ {:?}", robot_id, (x, y));
                        }
                    } else {
                        warn!("Robot: {} Collect failed (capacity?) @ {:?}", robot_id, (x, y));
                        if self.state.is_full() {
                            self.state.status = RobotStatus::ReturningToStation;
                            // self.current_target_coords = None;
                        }
                    }
                } else {
                    debug!("Robot: {} Resource changed pre-write @ {:?}", robot_id, (x, y));
                }
            } else {
                debug!("Robot: {} Resource gone pre-write @ {:?}", robot_id, (x, y));
            }
        }
        if remove_successful {
            self.knowledge.update_tile(x, y, TileInfo::Walkable);
            let event = RobotEvent::CollectionData {
                id: robot_id,
                x,
                y,
                resource_type: Some(target_type.clone()),
                amount: amount_collected,
            };
            if let Err(e) = sender.send(event) {
                error!("Robot: {} Failed send CollectionData: {}.", robot_id, e);
            }
        }
        remove_successful
    }

    fn update_knowledge_around(&mut self, map: &Arc<RwLock<Map>>) {
        let map_read_guard = match map.read() {
            Ok(g) => g,
            Err(p) => {
                error!("Robot: {} Map read poisoned! {}", self.state.id, p);
                return;
            }
        };
        let map_read = &*map_read_guard;
        let (x, y) = (self.state.x, self.state.y);
        self.knowledge.observe_and_update(x, y, map_read);

        for dir in Direction::all().iter() {
            let (nx, ny) = movement::next_position(x, y, dir, map_read);
            if (nx, ny) != (x, y) {
                self.knowledge.observe_and_update(nx, ny, map_read);
            }
        }
    }

    fn choose_best_explore_direction(&self, map: &Map) -> Direction {
        let directions = Direction::all();
        let mut best_direction = Direction::random();
        let mut best_score = -1;

        for dir in directions {
            let (nx, ny) = movement::next_position(self.state.x, self.state.y, &dir, map);
            if movement::is_valid_move(nx, ny, map)
                && !matches!(
                    self.knowledge.get_tile(nx, ny),
                    knowledge::TileInfo::Obstacle
                )
            {
                let score = match self.knowledge.get_tile(nx, ny) {
                    knowledge::TileInfo::Unknown => 2,
                    knowledge::TileInfo::Walkable => 1,
                    _ => 0,
                };
                if score > best_score {
                    best_score = score;
                    best_direction = dir;
                }
            }
        }
        best_direction
    }

    fn try_move(
        &mut self,
        direction: Direction,
        map: &Arc<RwLock<Map>>,
        config: &config::RobotTypeConfig,
        sender: &Sender<RobotEvent>, 
    ) {
        let map_read_guard = match map.read() {
            Ok(g) => g,
            Err(p) => {
                error!("Robot: {} Map read poisoned! {}", self.state.id, p);
                return;
            }
        };
        let map_read = &*map_read_guard;
        let (new_x, new_y) =
            movement::next_position(self.state.x, self.state.y, &direction, map_read);

        if movement::is_valid_move(new_x, new_y, map_read)
            && !matches!(
                self.knowledge.get_tile(new_x, new_y),
                knowledge::TileInfo::Obstacle
            )
        {
            debug!(
                "Robot: {} Moving from {:?} to {:?} (capacity: {}, energy: {})",
                self.state.id,
                (self.state.x, self.state.y),
                (new_x, new_y),
                self.state.max_capacity,
                self.state.energy
            );

            if self.state.energy >= config.movement_energy_cost {
                self.state.x = new_x;
                self.state.y = new_y;
                self.state.use_energy(config.movement_energy_cost);

                // Send position update to App/UI
                let _ = sender.send(RobotEvent::CollectionData {
                    id: self.state.id,
                    x: self.state.x,
                    y: self.state.y,
                    resource_type: None,
                    amount: 0,
                });                
            } else {
                warn!(
                    "Robot: {} Not enough energy to movEnergy: {}/{}",
                    self.state.id, self.state.energy, config.movement_energy_cost
                );
                self.state.status = RobotStatus::ReturningToStation;
                self.current_target_coords = None;
            }
        } else {
            debug!(
                "Robot: {} Move to {:?} blocked or invalid.",
                self.state.id,
                (new_x, new_y)
            );
        }
    }

    fn handle_returning_to_station(
        &mut self,
        sender: &Sender<RobotEvent>,
        map: &Arc<RwLock<Map>>,
        station_coords: (usize, usize),
        config: &config::RobotTypeConfig,
    ) {
        let robot_id = self.state.id;
        let (station_x, station_y) = station_coords;
        if self.state.x == station_x && self.state.y == station_y {
            info!("Robot: {} Arrived station.", robot_id);
            self.state.status = RobotStatus::AtStation;
            let k_clone = self.knowledge.clone();
            let ev = RobotEvent::ArrivedAtStation {
                id: robot_id,
                knowledge: k_clone,
            };
            if let Err(e) = sender.send(ev) {
                error!("Robot: {} Failed send Arrived: {}", robot_id, e);
                return;
            }
            info!("Robot: {} Waiting MergeComplete...", robot_id);

            match self
                .merge_complete_receiver
                .recv_timeout(config::MERGE_TIMEOUT)
            {
                Ok(RobotEvent::MergeComplete {
                    merged_knowledge, ..
                }) => {
                    info!("Robot: {} MergeComplete OK.", robot_id);
                    self.knowledge = merged_knowledge;
                    self.state.energy = self.state.max_energy;
                    self.state.collected_resources.clear();
                    self.state.status = RobotStatus::Collecting;
                    info!("Robot: {} Resuming collection.", robot_id);
                }
                Ok(o) => {
                    warn!("Robot: {} Unexpected event: {:?}", robot_id, o);
                    self.state.status = RobotStatus::Collecting;
                }
                Err(RecvTimeoutError::Timeout) => {
                    warn!("Robot: {} Merge Timeout.", robot_id);
                    self.state.status = RobotStatus::Collecting;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    error!("Robot: {} Merge channel disconnected.", robot_id);
                }
            }
            return;
        }

        let map_read_guard = match map.read() {
            Ok(g) => g,
            Err(p) => {
                error!("Robot: {} Map read poisoned! {}", robot_id, p);
                return;
            }
        };
        let map_read = &*map_read_guard;

        let direction = common::move_towards_target(
            self.state.x,
            self.state.y,
            station_x,
            station_y,
            &self.knowledge,
            map_read,
        );

        let (new_x, new_y) =
            movement::next_position(self.state.x, self.state.y, &direction, map_read);

        let mut moved = false;
        if movement::is_valid_move(new_x, new_y, map_read)
            && !matches!(
                self.knowledge.get_tile(new_x, new_y),
                knowledge::TileInfo::Obstacle
            )
        {
            self.state.x = new_x;
            self.state.y = new_y;
            self.state.use_energy(config.movement_energy_cost);
            moved = true;
        }

        if !moved {
            for _ in 0..RANDOM_MOVE_ATTEMPTS {
                let rd = movement::Direction::random();
                let (rx, ry) = movement::next_position(self.state.x, self.state.y, &rd, map_read);
                if movement::is_valid_move(rx, ry, map_read)
                    && !matches!(
                        self.knowledge.get_tile(rx, ry),
                        knowledge::TileInfo::Obstacle
                    )
                {
                    self.state.x = rx;
                    self.state.y = ry;
                    self.state.use_energy(config.movement_energy_cost);
                    moved = true;
                    break;
                }
            }
        }
        if !moved {
            debug!(
                "Robot: {} Path to station blocked @ {:?}.",
                robot_id,
                (self.state.x, self.state.y)
            );
        }

        thread::sleep(config::random_sleep_duration(
            config::RETURN_SLEEP_MIN_MS,
            config::RETURN_SLEEP_MAX_MS,
        ));
    }

    fn handle_at_station(&mut self) {
        thread::sleep(Duration::from_millis(config::AT_STATION_SLEEP_MS));
    }
}
