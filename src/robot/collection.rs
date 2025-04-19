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
                "Robot {}: Seting target resource type to {:?}",
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
        self.knowledge
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
            .map(|(coords, _)| coords)
    }

    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        let robot_id = self.state.id;
        let station_coords = self.knowledge.get_station_coords();
        let config = self.config.clone();
        let collection_action_cost = config
            .action_energy_cost
            .expect("Collection config must have action cost");

        thread::spawn(move || {
            info!("Robot {}: Starting collection thread", robot_id);

            loop {
                match self.state.status {
                    RobotStatus::Collecting => {
                        if self.state.energy <= config.low_energy_threshold {
                            info!("R{}: Low E, returnin", robot_id);
                            self.state.status = RobotStatus::ReturningToStation;
                            self.current_target_coords = None;
                            continue;
                        }
                        if self.state.is_full() {
                            info!("R{}: Full, returnin", robot_id);
                            self.state.status = RobotStatus::ReturningToStation;
                            self.current_target_coords = None;
                            continue;
                        }

                        let mut collected_this_turn = false;
                        let current_x = self.state.x;
                        let current_y = self.state.y;

                        if let Some(target_type) = &self.target_resource_type {
                            let resource_present = {
                                /* ... read lock scope ... */
                                let guard = match map.read() {
                                    Ok(g) => g,
                                    Err(p) => {
                                        error!("R{}: Map read poisoned! {}", robot_id, p);
                                        break;
                                    }
                                };
                                guard
                                    .get_resource(current_x, current_y)
                                    .map_or(false, |(rt, amount)| rt == *target_type && amount > 0)
                            };
                            if resource_present {
                                if self.state.use_energy(collection_action_cost) {
                                    let mut amount_collected = 0;
                                    let mut remove_successful = false;
                                    {
                                        /* ... write lock scope ... */
                                        let mut guard = match map.write() {
                                            Ok(g) => g,
                                            Err(p) => {
                                                error!("R{}: Map write poisoned! {}", robot_id, p);
                                                break;
                                            }
                                        };
                                        if let Some((res_type, amount)) =
                                            guard.get_resource(current_x, current_y)
                                        {
                                            if res_type == *target_type && amount > 0 {
                                                if self
                                                    .state
                                                    .collect_resource(target_type.clone(), amount)
                                                {
                                                    amount_collected = amount;
                                                    if guard
                                                        .remove_resource(current_x, current_y)
                                                        .is_some()
                                                    {
                                                        remove_successful = true;
                                                        info!(
                                                            "R{}: Colllected/removed {} {:?} @ {:?}",
                                                            robot_id,
                                                            amount_collected,
                                                            target_type,
                                                            (current_x, current_y)
                                                        );
                                                    } else {
                                                        error!(
                                                            "R{}: Failed remove map @ {:?}",
                                                            robot_id,
                                                            (current_x, current_y)
                                                        );
                                                    }
                                                } else {
                                                    warn!(
                                                        "R{}: Collect failed (capacity?) @ {:?}",
                                                        robot_id,
                                                        (current_x, current_y)
                                                    );
                                                }
                                            } else {
                                                debug!(
                                                    "R{}: Resource changd pre-write @ {:?}",
                                                    robot_id,
                                                    (current_x, current_y)
                                                );
                                            }
                                        } else {
                                            debug!(
                                                "R{}: Resource gone pre-write @ {:?}",
                                                robot_id,
                                                (current_x, current_y)
                                            );
                                        }
                                    }
                                    if remove_successful {
                                        collected_this_turn = true;
                                        self.knowledge.update_tile(
                                            current_x,
                                            current_y,
                                            TileInfo::Walkable,
                                        );
                                        let event = RobotEvent::CollectionData {
                                            id: robot_id,
                                            x: current_x,
                                            y: current_y,
                                            resource_type: Some(target_type.clone()),
                                            amount: amount_collected,
                                        };
                                        if let Err(e) = sender.send(event) {
                                            error!(
                                                "R{}: Failed send CollectionData: {}.",
                                                robot_id, e
                                            );
                                            break;
                                        }
                                    }
                                } else {
                                    warn!(
                                        "R{}: No energy ({}) to collect @ {:?}",
                                        robot_id,
                                        self.state.energy,
                                        (current_x, current_y)
                                    );
                                }
                            }
                        }

                        if !collected_this_turn {
                            let map_read_guard = match map.read() {
                                Ok(g) => g,
                                Err(p) => {
                                    error!("R{}: Map read poisoned! {}", robot_id, p);
                                    break;
                                }
                            };
                            let map_read = &*map_read_guard;
                            {
                                let x = self.state.x;
                                let y = self.state.y;
                                let knowledge: &mut RobotKnowledge = &mut self.knowledge;
                                knowledge.observe_and_update(x, y, map_read);

                                for dir in Direction::all().iter() {
                                    let (nx, ny) = movement::next_position(x, y, dir, map_read);

                                    if (nx, ny) != (x, y) {
                                        knowledge.observe_and_update(nx, ny, map_read);
                                    }
                                }
                            };

                            let direction =
                                if let Some(target_coords) = self.find_nearest_target_resource() {
                                    debug!(
                                        "R{}: Moving towards {:?} @ {:?}",
                                        robot_id,
                                        self.target_resource_type.as_ref().unwrap(),
                                        target_coords
                                    );
                                    self.current_target_coords = Some(target_coords);

                                    common::move_towards_target(
                                        self.state.x,
                                        self.state.y,
                                        target_coords.0,
                                        target_coords.1,
                                        &self.knowledge,
                                        map_read,
                                    )
                                } else {
                                    debug!(
                                        "R{}: No target {:?}. Enhanced exploring.",
                                        robot_id, self.target_resource_type
                                    );
                                    self.current_target_coords = None;

                                    let directions = Direction::all();
                                    let mut best_direction = Direction::random();
                                    let mut best_score = -1;

                                    for dir in directions {
                                        let (nx, ny) = movement::next_position(
                                            self.state.x,
                                            self.state.y,
                                            &dir,
                                            map_read,
                                        );
                                        if movement::is_valid_move(nx, ny, map_read)
                                            && !matches!(
                                                self.knowledge.get_tile(nx, ny),
                                                knowledge::TileInfo::Obstacle
                                            )
                                        {
                                            let score = match self.knowledge.get_tile(nx, ny) {
                                                knowledge::TileInfo::Unknown => 2, // Prefer unexplored tiles
                                                knowledge::TileInfo::Walkable => 1, // Already explored but walkable is okay
                                                _ => 0,
                                            };
                                            if score > best_score {
                                                best_score = score;
                                                best_direction = dir;
                                            }
                                        }
                                    }

                                    best_direction
                                };

                            let (new_x, new_y) = movement::next_position(
                                self.state.x,
                                self.state.y,
                                &direction,
                                map_read,
                            );

                            if movement::is_valid_move(new_x, new_y, map_read) {
                                if !matches!(
                                    self.knowledge.get_tile(new_x, new_y),
                                    knowledge::TileInfo::Obstacle
                                ) {
                                    debug!(
                                        "R{}: Moving from {:?} to {:?}",
                                        robot_id,
                                        (self.state.x, self.state.y),
                                        (new_x, new_y)
                                    );
                                    if self.state.energy >= config.movement_energy_cost {
                                        self.state.x = new_x;
                                        self.state.y = new_y;
                                        self.state.use_energy(config.movement_energy_cost);
                                    } else {
                                        warn!(
                                            "R{}: Not enough energy to move: {}/{}",
                                            robot_id,
                                            self.state.energy,
                                            config.movement_energy_cost
                                        );
                                        self.state.status = RobotStatus::ReturningToStation;
                                        self.current_target_coords = None;
                                    }
                                } else {
                                    debug!(
                                        "R{}: Move to {:?} blocked by known obstacle.",
                                        robot_id,
                                        (new_x, new_y)
                                    );
                                }
                            } else {
                                debug!(
                                    "R{}: Move to {:?} would be invalid.",
                                    robot_id,
                                    (new_x, new_y)
                                );
                            }

                            drop(map_read_guard);
                        }

                        thread::sleep(config::random_sleep_duration(
                            config.primary_action_sleep_min_ms,
                            config.primary_action_sleep_max_ms,
                        ));
                    }

                    RobotStatus::ReturningToStation => {
                        let (station_x, station_y) = station_coords;
                        if self.state.x == station_x && self.state.y == station_y {
                            info!("R{}: Arrived station.", robot_id);
                            self.state.status = RobotStatus::AtStation;
                            let k_clone = self.knowledge.clone();
                            let ev = RobotEvent::ArrivedAtStation {
                                id: robot_id,
                                knowledge: k_clone,
                            };
                            if let Err(e) = sender.send(ev) {
                                error!("R{}: Failed send Arrived: {}", robot_id, e);
                                break;
                            };
                            info!("R{}: Waiting MergeComplete...", robot_id);

                            match self
                                .merge_complete_receiver
                                .recv_timeout(config::MERGE_TIMEOUT)
                            {
                                Ok(RobotEvent::MergeComplete {
                                    merged_knowledge, ..
                                }) => {
                                    info!("R{}: MergeComplete OK.", robot_id);
                                    self.knowledge = merged_knowledge;
                                    self.state.energy = config::RECHARGE_ENERGY;
                                    self.state.collected_resources.clear();
                                    self.state.status = RobotStatus::Collecting;
                                    info!("R{}: Resuming collection.", robot_id);
                                }
                                Ok(o) => {
                                    warn!("R{}: Unexpected event: {:?}", robot_id, o);
                                    self.state.status = RobotStatus::Collecting;
                                }
                                Err(RecvTimeoutError::Timeout) => {
                                    warn!("R{}: Merge Timeout.", robot_id);
                                    self.state.status = RobotStatus::Collecting;
                                }
                                Err(RecvTimeoutError::Disconnected) => {
                                    error!("R{}: Merge channel disconnected.", robot_id);
                                    break;
                                }
                            }

                            continue;
                        }

                        let map_read_guard = match map.read() {
                            Ok(g) => g,
                            Err(p) => {
                                error!("R{}: Map read poisoned! {}", robot_id, p);
                                break;
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

                        let (new_x, new_y) = movement::next_position(
                            self.state.x,
                            self.state.y,
                            &direction,
                            map_read,
                        );

                        let mut moved = false;
                        if movement::is_valid_move(new_x, new_y, map_read) {
                            if !matches!(
                                self.knowledge.get_tile(new_x, new_y),
                                knowledge::TileInfo::Obstacle
                            ) {
                                self.state.x = new_x;
                                self.state.y = new_y;
                                self.state.use_energy(config.movement_energy_cost);
                                moved = true;
                            }
                        }

                        if !moved {
                            for _ in 0..4 {
                                let rd = movement::Direction::random();
                                let (rx, ry) = movement::next_position(
                                    self.state.x,
                                    self.state.y,
                                    &rd,
                                    map_read,
                                );
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
                                "R{}: Path to station blocked @ {:?}.",
                                robot_id,
                                (self.state.x, self.state.y)
                            );
                        }

                        drop(map_read_guard);

                        thread::sleep(config::random_sleep_duration(
                            config::RETURN_SLEEP_MIN_MS,
                            config::RETURN_SLEEP_MAX_MS,
                        ));
                    }
                    RobotStatus::AtStation => {
                        thread::sleep(Duration::from_millis(config::AT_STATION_SLEEP_MS));
                    }
                    _ => {
                        error!("R{}: Unhandled state {:?}.", robot_id, self.state.status);
                        self.state.status = RobotStatus::Collecting;
                        thread::sleep(config::UNHANDLED_STATE_SLEEP);
                    }
                }
            }
            info!("Robot {}: Thread shutting down.", robot_id);
            if sender
                .send(RobotEvent::Shutdown {
                    id: robot_id,
                    reason: "Thread loop exited".to_string(),
                })
                .is_err()
            {
                error!("R{}: Failed send final shutdown.", robot_id);
            }
        });
    }
}
