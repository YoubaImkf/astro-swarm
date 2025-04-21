use log::{debug, error, info, warn};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::communication::channels::{ResourceType, RobotEvent};
use crate::map::noise::Map;
use crate::robot::movement::Direction;
use crate::robot::state::RobotStatus;

use super::knowledge::{self, RobotKnowledge, TileInfo};
use super::{common, config, movement, RobotState};

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub science_bonus: u32,
    pub energy_cost: u32, // Passive energy cost per move
}

pub struct ScientificRobot {
    state: RobotState,
    modules: Vec<Module>,
    knowledge: RobotKnowledge,
    merge_complete_receiver: Receiver<RobotEvent>,
    config: config::RobotTypeConfig,
}

impl ScientificRobot {
    pub fn new(
        initial_state: RobotState,
        map_width: usize,
        map_height: usize,
        merge_complete_receiver: Receiver<RobotEvent>,
    ) -> Self {
        Self {
            knowledge: RobotKnowledge::new(map_width, map_height),
            state: initial_state,
            modules: Vec::new(),
            merge_complete_receiver,
            config: config::SCIENTIFIC_CONFIG.clone(),
        }
    }

    pub fn add_module(&mut self, name: &str, science_bonus: u32, energy_cost: u32) {
        info!(
            "Robot {}: Adding module '{}' (Bonus: {}, Cost: {})",
            self.state.id, name, science_bonus, energy_cost
        );
        self.modules.push(Module {
            name: name.to_string(),
            science_bonus,
            energy_cost,
        });
    }

    fn analyze_science_point(&self, base_value: u32) -> u32 {
        let module_bonus: u32 = self.modules.iter().map(|module| module.science_bonus).sum();
        base_value.saturating_add(module_bonus)
    }

    fn get_module_passive_energy_cost(&self) -> u32 {
        self.modules.iter().map(|m| m.energy_cost).sum()
    }

    fn find_nearest_known_science_point(&self) -> Option<(usize, usize)> {
        self.knowledge
            .map
            .iter()
            .filter_map(|(&(x, y), tile_info)| {
                if matches!(
                    tile_info,
                    TileInfo::Resource(ResourceType::SciencePoints, _)
                ) {
                    let dist_sq = (x as isize - self.state.x as isize).pow(2)
                        + (y as isize - self.state.y as isize).pow(2);
                    Some(((x, y), dist_sq))
                } else {
                    None
                }
            })
            .min_by_key(|&(_, dist_sq)| dist_sq)
            .map(|(coords, _)| coords)
    }

    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        let robot_id = self.state.id;
        let station_coords = self.knowledge.get_station_coords();
        let config = self.config.clone();
        let analysis_action_cost = config
            .action_energy_cost
            .expect("Scientific config must have an  action cost");

        thread::spawn(move || {
            let mut visited_in_cycle: HashSet<(usize, usize)> = HashSet::new();
            info!("Robot {}: Starting scientific analysis thread.", robot_id);

            loop {
                let passive_module_cost = self.get_module_passive_energy_cost();

                match self.state.status {
                    RobotStatus::Analyzing => {
                        if self.state.energy <= config.low_energy_threshold {
                            info!("Robot: {} Low E ({}), returning.", robot_id, self.state.energy);
                            self.state.status = RobotStatus::ReturningToStation;
                            visited_in_cycle.clear();
                            continue;
                        }

                        let map_read_guard = match map.read() {
                            Ok(g) => g,
                            Err(p) => {
                                error!("Robot: {} Map read poisoned! {}", robot_id, p);
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

                        let mut analyzed_this_turn = false;
                        let current_x = self.state.x;
                        let current_y = self.state.y;

                        if let TileInfo::Resource(ResourceType::SciencePoints, base_amount) =
                            self.knowledge.get_tile(current_x, current_y)
                        {
                            if *base_amount > 0 {
                                let analysis_total_cost =
                                    analysis_action_cost.saturating_add(passive_module_cost);
                                if self.state.use_energy(analysis_total_cost) {
                                    let science_value = self.analyze_science_point(*base_amount);
                                    info!(
                                        "Robot: {} Analyzed science point at {:?}, valuEnergy: {}",
                                        robot_id,
                                        (current_x, current_y),
                                        science_value
                                    );
                                    analyzed_this_turn = true;

                                    if !self.state.collect_resource(
                                        ResourceType::SciencePoints,
                                        science_value,
                                    ) {
                                        warn!("Robot: {} Failed to record science value (internal capacity?), valuEnergy: {}", robot_id, science_value);
                                    }

                                    let event = RobotEvent::ScienceData {
                                        id: robot_id,
                                        x: current_x,
                                        y: current_y,
                                        resource_type: ResourceType::SciencePoints,
                                        amount: science_value,
                                        modules: self
                                            .modules
                                            .iter()
                                            .map(|m| m.name.clone())
                                            .collect(),
                                    };
                                    if let Err(e) = sender.send(event) {
                                        error!("Robot: {} Failed send ScienceData: {}.", robot_id, e);
                                        drop(map_read_guard);
                                        break;
                                    }
                                } else {
                                    warn!(
                                        "Robot: {} Not enough energy ({}) for analysis @ {:?}",
                                        robot_id,
                                        self.state.energy,
                                        (current_x, current_y)
                                    );
                                }
                            }
                        }

                        if !analyzed_this_turn {
                            let move_total_cost = config
                                .movement_energy_cost
                                .saturating_add(passive_module_cost);
                            if !self.state.use_energy(move_total_cost) {
                                warn!(
                                    "Robot: {} Not enough energy ({}) to move. Returning.",
                                    robot_id, self.state.energy
                                );
                                self.state.status = RobotStatus::ReturningToStation;
                                visited_in_cycle.clear();
                                drop(map_read_guard);
                                continue;
                            }

                            let direction = if let Some(target_coords) =
                                self.find_nearest_known_science_point()
                            {
                                debug!(
                                    "Robot: {} Moving towards known Science Point @ {:?}",
                                    robot_id, target_coords
                                );
                                common::move_towards_target(
                                    self.state.x,
                                    self.state.y,
                                    target_coords.0,
                                    target_coords.1,
                                    &self.knowledge,
                                    map_read,
                                )
                            } else {
                                debug!("Robot: {} No known Science Points. Exploring.", robot_id);
                                movement::smart_direction(
                                    self.state.x,
                                    self.state.y,
                                    &self.knowledge,
                                    &visited_in_cycle,
                                    map_read,
                                )
                                .unwrap_or_else(movement::Direction::random)
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
                                    self.state.x = new_x;
                                    self.state.y = new_y;
                                    visited_in_cycle.insert((new_x, new_y));
                                } else {
                                    debug!(
                                        "Robot: {} Move {:?} blocked by known obstacle.",
                                        robot_id,
                                        (new_x, new_y)
                                    );
                                }
                            }
                        }

                        drop(map_read_guard);

                        thread::sleep(config::random_sleep_duration(
                            config.primary_action_sleep_min_ms,
                            config.primary_action_sleep_max_ms,
                        ));
                    }

                    RobotStatus::ReturningToStation => {
                        let (station_x, station_y) = station_coords;
                        if self.state.x == station_x && self.state.y == station_y {
                            info!("Robot: {} Arrived atr station", robot_id);
                            self.state.status = RobotStatus::AtStation;
                            let k_clone = self.knowledge.clone();
                            let ev = RobotEvent::ArrivedAtStation {
                                id: robot_id,
                                knowledge: k_clone,
                            };
                            if let Err(e) = sender.send(ev) {
                                error!("Robot: {} Failed send Arrived: {}", robot_id, e);
                                break;
                            };
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
                                    self.state.energy = config::RECHARGE_ENERGY;
                                    self.state
                                        .collected_resources
                                        .remove(&ResourceType::SciencePoints);
                                    self.state.status = RobotStatus::Analyzing;
                                    info!("Robot: {} Resuming analysis.", robot_id);
                                }
                                Ok(o) => {
                                    warn!("Robot: {} Unexpected event: {:?}", robot_id, o);
                                    self.state.status = RobotStatus::Analyzing;
                                }
                                Err(RecvTimeoutError::Timeout) => {
                                    warn!("Robot: {} Merge Timeout.", robot_id);
                                    self.state.status = RobotStatus::Analyzing;
                                }
                                Err(RecvTimeoutError::Disconnected) => {
                                    error!("Robot: {} Merge channel disconnected.", robot_id);
                                    break;
                                }
                            }
                            continue;
                        }

                        // Move to station..
                        let move_total_cost = config
                            .movement_energy_cost
                            .saturating_add(passive_module_cost);
                        if !self.state.use_energy(move_total_cost) {
                            warn!(
                                "Robot: {} Not enough energy ({}) to return to station! Waiting.",
                                robot_id, self.state.energy
                            );
                            thread::sleep(Duration::from_secs(3));
                            continue;
                        }

                        let map_read_guard = match map.read() {
                            Ok(g) => g,
                            Err(p) => {
                                error!("Robot: {} Map read poisoned! {}", robot_id, p);
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
                        drop(map_read_guard);

                        thread::sleep(config::random_sleep_duration(
                            config::RETURN_SLEEP_MIN_MS,
                            config::RETURN_SLEEP_MAX_MS,
                        ));
                    }
                    RobotStatus::AtStation => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    _ => {
                        error!(
                            "Robot: {} In unhandld statde {:?}. Defaulting to Analyzing.",
                            robot_id, self.state.status
                        );
                        self.state.status = RobotStatus::Analyzing;
                        thread::sleep(config::UNHANDLED_STATE_SLEEP);
                    }
                }
            }

            info!("Robot {}: Thread shutting down", robot_id);
            if sender
                .send(RobotEvent::Shutdown {
                    id: robot_id,
                    reason: "Thread loop exited".to_string(),
                })
                .is_err()
            {
                error!("Robot {}: Failed send final shutdown", robot_id);
            }
        });
    }
}
