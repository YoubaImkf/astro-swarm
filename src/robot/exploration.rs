use log::{debug, error, info, warn};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use super::knowledge::{self, RobotKnowledge, TileInfo};
use super::{common, config, movement, RobotState};
use crate::communication::channels::RobotEvent;
use crate::map::noise::Map;
use crate::robot::movement::Direction;
use crate::robot::state::RobotStatus;

pub struct ExplorationRobot {
    state: RobotState,
    knowledge: RobotKnowledge,
    merge_complete_receiver: Receiver<RobotEvent>,
    config: config::RobotTypeConfig,
}

impl ExplorationRobot {
    pub fn new(
        initial_state: RobotState,
        map_width: usize,
        map_height: usize,
        merge_complete_receiver: Receiver<RobotEvent>,
    ) -> Self {
        Self {
            knowledge: RobotKnowledge::new(map_width, map_height),
            state: initial_state,
            merge_complete_receiver,
            config: config::EXPLORATION_CONFIG.clone(),
        }
    }

    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        let robot_id = self.state.id;
        let station_coords = self.knowledge.get_station_coords();
        let config = self.config.clone();

        thread::spawn(move || {
            let mut visited_during_exploration: HashSet<(usize, usize)> = HashSet::new();
            info!("Robot {}: Starting exploration thread.", robot_id);

            loop {
                match self.state.status {
                    RobotStatus::Exploring => {
                        if self.state.energy <= config.low_energy_threshold {
                            info!(
                                "Robot {}: Low energy ({}), returning to station.",
                                self.state.id, self.state.energy
                            );
                            self.state.status = RobotStatus::ReturningToStation;
                            visited_during_exploration.clear();
                            continue;
                        }

                        let map_read_guard = match map.read() {
                            Ok(guard) => guard,
                            Err(poisoned) => {
                                error!(
                                    "Robot {}: Map lock poisoned! Shutting down. Err: {}",
                                    self.state.id, poisoned
                                );
                                let _ = sender.send(RobotEvent::Shutdown {
                                    id: self.state.id,
                                    reason: "Map lock poisoned".to_string(),
                                });
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

                        let direction = movement::smart_direction(
                            self.state.x,
                            self.state.y,
                            &self.knowledge,
                            &visited_during_exploration,
                            map_read,
                        )
                        .unwrap_or_else(movement::Direction::random);

                        let (new_x, new_y) = movement::next_position(
                            self.state.x,
                            self.state.y,
                            &direction,
                            map_read,
                        );

                        let moved = if movement::is_valid_move(new_x, new_y, map_read) {
                            if !matches!(self.knowledge.get_tile(new_x, new_y), TileInfo::Obstacle)
                            {
                                self.state.x = new_x;
                                self.state.y = new_y;
                                visited_during_exploration.insert((new_x, new_y));
                                self.state.use_energy(config.movement_energy_cost);
                                true
                            } else {
                                debug!("Robot: {} Move {:?} blocked.", robot_id, (new_x, new_y));
                                false
                            }
                        } else {
                            false
                        };

                        let is_obstacle_at_new_pos =
                            map_read.is_obstacle(self.state.x, self.state.y);
                        drop(map_read_guard);

                        if moved {
                            let event = RobotEvent::ExplorationData {
                                id: self.state.id,
                                x: self.state.x,
                                y: self.state.y,
                                is_obstacle: is_obstacle_at_new_pos,
                            };
                            if let Err(e) = sender.send(event) {
                                error!(
                                    "Robot {}: Failed to send ExplorationData: {}. Shutting down.",
                                    self.state.id, e
                                );
                                break;
                            }
                        }

                        thread::sleep(config::random_sleep_duration(
                            config.primary_action_sleep_min_ms,
                            config.primary_action_sleep_max_ms,
                        ));
                    }

                    RobotStatus::ReturningToStation => {
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
                                    self.state.energy = self.state.max_energy;
                                    self.state.status = RobotStatus::Exploring;
                                    visited_during_exploration.clear();
                                    info!("Robot: {} Resuming exploration.", robot_id);
                                }
                                Ok(o) => {
                                    warn!("Robot: {} Unexpected event: {:?}", robot_id, o);
                                    self.state.status = RobotStatus::Exploring;
                                }
                                Err(RecvTimeoutError::Timeout) => {
                                    warn!("Robot: {} Merge Timeout.", robot_id);
                                    self.state.status = RobotStatus::Exploring;
                                }
                                Err(RecvTimeoutError::Disconnected) => {
                                    error!("Robot: {} Merge channel disconnected.", robot_id);
                                    break;
                                }
                            }

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
                                "Robot: {} Path to station blocked @ {:?}.",
                                robot_id,
                                (self.state.x, self.state.y)
                            );
                        }
                        drop(map_read_guard);
                        debug!(
                            "Robot: {} Returning @ {:?}, Energy: {}",
                            robot_id,
                            (self.state.x, self.state.y),
                            self.state.energy
                        );
                        thread::sleep(config::random_sleep_duration(
                            config::RETURN_SLEEP_MIN_MS,
                            config::RETURN_SLEEP_MAX_MS,
                        ));
                    }
                    RobotStatus::AtStation => {
                        thread::sleep(Duration::from_millis(config::AT_STATION_SLEEP_MS));
                    } // Use config
                    _ => {
                        error!("Robot: {} Unhandle state {:?}.", robot_id, self.state.status);
                        self.state.status = RobotStatus::Exploring;
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
                error!("Robot: {} Failed send final shutdown.", robot_id);
            }
        });
    }
}
