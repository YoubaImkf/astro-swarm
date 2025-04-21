use log::{debug, error, info, warn};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::communication::channels::{ResourceType, RobotEvent};
use crate::map::noise::Map;
use crate::robot::core::movement::Direction;
use crate::robot::core::state::RobotStatus;

use crate::robot::core::knowledge::{RobotKnowledge, TileInfo};
use crate::robot::core::movement;
use crate::robot::utils::{common, config};
use crate::robot::RobotState;

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
            .expect("Scientific config must have an action cost");

        thread::spawn(move || {
            let mut visited_in_cycle: HashSet<(usize, usize)> = HashSet::new();
            info!("Robot {}: Starting scientific analysis thread.", robot_id);

            loop {
                let passive_module_cost = self.get_module_passive_energy_cost();

                match self.state.status {
                    RobotStatus::Analyzing => {
                        if self.state.energy <= config.low_energy_threshold {
                            info!(
                                "Robot: {} Low energy ({}), returning.",
                                robot_id, self.state.energy
                            );
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

                        self.update_knowledge_around(&map_read);

                        if self.try_analyze_current_tile(
                            &sender,
                            analysis_action_cost,
                            passive_module_cost,
                        ) {
                            // Analysis done, sleep and continue
                            drop(map_read_guard);
                            thread::sleep(config::random_sleep_duration(
                                config.primary_action_sleep_min_ms,
                                config.primary_action_sleep_max_ms,
                            ));
                            continue;
                        }

                        if !self.try_move_towards_science(
                            &sender,
                            &map_read,
                            &mut visited_in_cycle,
                            passive_module_cost,
                            &config,
                        ) {
                            // Could not move, maybe blocked or out of energy
                            drop(map_read_guard);
                            continue;
                        }

                        drop(map_read_guard);
                        thread::sleep(config::random_sleep_duration(
                            config.primary_action_sleep_min_ms,
                            config.primary_action_sleep_max_ms,
                        ));
                    }

                    RobotStatus::ReturningToStation => {
                        if self.handle_returning_to_station(
                            &sender,
                            &map,
                            station_coords,
                            // passive_module_cost,
                            // &config,
                        ) {
                            continue;
                        }
                    }

                    RobotStatus::AtStation => {
                        thread::sleep(Duration::from_millis(100));
                    }

                    _ => {
                        error!(
                            "Robot: {} In unhandled state {:?}. Defaulting to Analyzing.",
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

    fn update_knowledge_around(&mut self, map: &Map) {
        let (x, y) = (self.state.x, self.state.y);
        self.knowledge.observe_and_update(x, y, map);
        for dir in Direction::all().iter() {
            let (nx, ny) = movement::next_position(x, y, dir, map);
            if (nx, ny) != (x, y) {
                self.knowledge.observe_and_update(nx, ny, map);
            }
        }
    }

    fn try_analyze_current_tile(
        &mut self,
        sender: &Sender<RobotEvent>,
        analysis_action_cost: u32,
        passive_module_cost: u32,
    ) -> bool {
        let (current_x, current_y) = (self.state.x, self.state.y);
        if let TileInfo::Resource(ResourceType::SciencePoints, base_amount) =
            self.knowledge.get_tile(current_x, current_y)
        {
            if *base_amount > 0 {
                let analysis_total_cost = analysis_action_cost.saturating_add(passive_module_cost);
                if self.state.use_energy(analysis_total_cost) {
                    let science_value = self.analyze_science_point(*base_amount);
                    info!(
                        "Robot: {} Analyzed science point at {:?}, value: {}",
                        self.state.id,
                        (current_x, current_y),
                        science_value
                    );
                    if !self
                        .state
                        .collect_resource(ResourceType::SciencePoints, science_value)
                    {
                        warn!(
                            "Robot: {} Failed to record science value (internal capacity?), value: {}",
                            self.state.id, science_value
                        );
                        if self.state.is_full() {
                            self.state.status = RobotStatus::ReturningToStation;
                        }
                    }
                    let event = RobotEvent::ScienceData {
                        id: self.state.id,
                        x: current_x,
                        y: current_y,
                        resource_type: ResourceType::SciencePoints,
                        amount: science_value,
                        modules: self.modules.iter().map(|m| m.name.clone()).collect(),
                    };
                    let _ = sender.send(event);
                    return true;
                } else {
                    warn!(
                        "Robot: {} Not enough energy ({}) for analysis @ {:?}",
                        self.state.id,
                        self.state.energy,
                        (current_x, current_y)
                    );
                }
            }
        }
        false
    }

    fn try_move_towards_science(
        &mut self,
        sender: &Sender<RobotEvent>,
        map: &Map,
        visited_in_cycle: &mut HashSet<(usize, usize)>,
        passive_module_cost: u32,
        config: &config::RobotTypeConfig,
    ) -> bool {
        let move_total_cost = config
            .movement_energy_cost
            .saturating_add(passive_module_cost);
        if !self.state.use_energy(move_total_cost) {
            warn!(
                "Robot: {} Not enough energy ({}) to move. Returning.",
                self.state.id, self.state.energy
            );
            self.state.status = RobotStatus::ReturningToStation;
            visited_in_cycle.clear();
            return false;
        }

        let direction = if let Some(target_coords) = self.find_nearest_known_science_point() {
            debug!(
                "Robot: {} Moving towards known Science Point @ {:?}",
                self.state.id, target_coords
            );
            common::move_towards_target(
                self.state.x,
                self.state.y,
                target_coords.0,
                target_coords.1,
                &self.knowledge,
                map,
            )
        } else {
            debug!(
                "Robot: {} No known Science Points. Exploring.",
                self.state.id
            );
            movement::smart_direction(
                self.state.x,
                self.state.y,
                &self.knowledge,
                visited_in_cycle,
                map,
            )
            .unwrap_or_else(movement::Direction::random)
        };

        let (new_x, new_y) = movement::next_position(self.state.x, self.state.y, &direction, map);

        if movement::is_valid_move(new_x, new_y, map)
            && !matches!(self.knowledge.get_tile(new_x, new_y), TileInfo::Obstacle)
        {
            self.state.x = new_x;
            self.state.y = new_y;
            visited_in_cycle.insert((new_x, new_y));
            let _ = sender.send(RobotEvent::ScienceData {
                id: self.state.id,
                x: self.state.x,
                y: self.state.y,
                resource_type: ResourceType::SciencePoints,
                amount: 0,
                modules: self.modules.iter().map(|m| m.name.clone()).collect(),
            });
            true
        } else {
            debug!(
                "Robot: {} Move {:?} blocked or invalid.",
                self.state.id,
                (new_x, new_y)
            );
            false
        }
    }

    fn handle_returning_to_station(
        &mut self,
        sender: &Sender<RobotEvent>,
        map: &Arc<RwLock<Map>>,
        station_coords: (usize, usize),
        // passive_module_cost: u32,
        // config: &config::RobotTypeConfig,
    ) -> bool {
        let (station_x, station_y) = station_coords;
        if self.state.x == station_x && self.state.y == station_y {
            info!("Robot: {} Arrived at station", self.state.id);
            self.state.status = RobotStatus::AtStation;
            let k_clone = self.knowledge.clone();
            let ev = RobotEvent::ArrivedAtStation {
                id: self.state.id,
                knowledge: k_clone,
            };
            let _ = sender.send(ev);
            info!("Robot: {} Waiting MergeComplete...", self.state.id);

            match self
                .merge_complete_receiver
                .recv_timeout(config::MERGE_TIMEOUT)
            {
                Ok(RobotEvent::MergeComplete {
                    merged_knowledge, ..
                }) => {
                    info!("Robot: {} MergeComplete OK.", self.state.id);
                    self.knowledge = merged_knowledge;
                    self.state.energy = self.state.max_energy;
                    self.state
                        .collected_resources
                        .remove(&ResourceType::SciencePoints);
                    self.state.status = RobotStatus::Analyzing;
                    info!("Robot: {} Resuming analysis.", self.state.id);
                }
                Ok(o) => {
                    warn!("Robot: {} Unexpected event: {:?}", self.state.id, o);
                    self.state.status = RobotStatus::Analyzing;
                }
                Err(RecvTimeoutError::Timeout) => {
                    warn!("Robot: {} Merge Timeout.", self.state.id);
                    self.state.status = RobotStatus::Analyzing;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    error!("Robot: {} Merge channel disconnected.", self.state.id);
                }
            }
            return true;
        }

        //let move_total_cost = config.movement_energy_cost.saturating_add(passive_module_cost);
        // if !self.state.use_energy(move_total_cost) {
        //     warn!(
        //         "Robot: {} Not enough energy ({}) to return to station! Waiting.",
        //         self.state.id, self.state.energy
        //     );
        //     thread::sleep(Duration::from_secs(3));
        //     return true;
        // }

        let map_read_guard = match map.read() {
            Ok(g) => g,
            Err(p) => {
                error!("Robot: {} Map read poisoned! {}", self.state.id, p);
                return true;
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
            && !matches!(self.knowledge.get_tile(new_x, new_y), TileInfo::Obstacle)
        {
            self.state.x = new_x;
            self.state.y = new_y;
            moved = true;
        }
        if !moved {
            for _ in 0..4 {
                let rd = movement::Direction::random();
                let (rx, ry) = movement::next_position(self.state.x, self.state.y, &rd, map_read);
                if movement::is_valid_move(rx, ry, map_read)
                    && !matches!(self.knowledge.get_tile(rx, ry), TileInfo::Obstacle)
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
                self.state.id,
                (self.state.x, self.state.y)
            );
        }
        drop(map_read_guard);

        thread::sleep(config::random_sleep_duration(
            config::RETURN_SLEEP_MIN_MS,
            config::RETURN_SLEEP_MAX_MS,
        ));
        true
    }
}
