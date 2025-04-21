use log::{debug, error, info, warn};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::robot::core::knowledge::{RobotKnowledge, TileInfo};
use crate::robot::utils::common;
use crate::robot::utils::config;
use crate::robot::core::movement;
use crate::robot::core::movement::Direction;
use crate::robot::core::state::{RobotState, RobotStatus};
use crate::communication::channels::RobotEvent;
use crate::map::noise::Map;

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

        thread::spawn(move || {
            let mut visited: HashSet<(usize, usize)> = HashSet::new();
            info!("Robot {}: Starting exploration thread.", robot_id);

            loop {
                match self.state.status {
                    RobotStatus::Exploring => {
                        if self.low_energy() {
                            self.transition_to_returning(&mut visited);
                            continue;
                        }
                        if let Err(e) = self.explore_step(&sender, &map, &mut visited) {
                            error!("Robot {}: {}", robot_id, e);
                            break;
                        }
                    }
                    RobotStatus::ReturningToStation => {
                        if self.handle_returning_to_station(&sender, &map, station_coords, &mut visited) {
                            continue;
                        }
                    }
                    RobotStatus::AtStation => {
                        thread::sleep(Duration::from_millis(config::AT_STATION_SLEEP_MS));
                    }
                    _ => {
                        error!("Robot: {} Unhandled state {:?}.", robot_id, self.state.status);
                        self.state.status = RobotStatus::Exploring;
                        thread::sleep(config::UNHANDLED_STATE_SLEEP);
                    }
                }
            }
            info!("Robot {}: Thread shutting down.", robot_id);
            let _ = sender.send(RobotEvent::Shutdown {
                id: robot_id,
                reason: "Thread loop exited".to_string(),
            });
        });
    }

    fn low_energy(&self) -> bool {
        self.state.energy <= self.config.low_energy_threshold
    }

    fn transition_to_returning(&mut self, visited: &mut HashSet<(usize, usize)>) {
        info!(
            "Robot {}: Low energy ({}), returning to station.",
            self.state.id, self.state.energy
        );
        self.state.status = RobotStatus::ReturningToStation;
        visited.clear();
    }

    fn explore_step(
        &mut self,
        sender: &Sender<RobotEvent>,
        map: &Arc<RwLock<Map>>,
        visited: &mut HashSet<(usize, usize)>,
    ) -> Result<(), String> {
        let map_read_guard = map.read().map_err(|e| format!("Map lock poisoned: {}", e))?;
        let map_read = &*map_read_guard;

        self.observe_surroundings(map_read);

        let direction = movement::smart_direction(
            self.state.x,
            self.state.y,
            &self.knowledge,
            visited,
            map_read,
        )
        .unwrap_or_else(movement::Direction::random);

        let (new_x, new_y) = movement::next_position(self.state.x, self.state.y, &direction, map_read);

        let moved = self.try_move(new_x, new_y, visited, map_read);

        let is_obstacle = map_read.is_obstacle(self.state.x, self.state.y);
        drop(map_read_guard);

        if moved {
            let event = RobotEvent::ExplorationData {
                id: self.state.id,
                x: self.state.x,
                y: self.state.y,
                is_obstacle,
            };
            sender.send(event).map_err(|e| format!("Failed to send ExplorationData: {}", e))?;
        }

        thread::sleep(config::random_sleep_duration(
            self.config.primary_action_sleep_min_ms,
            self.config.primary_action_sleep_max_ms,
        ));
        Ok(())
    }

    fn observe_surroundings(&mut self, map: &Map) {
        let (x, y) = (self.state.x, self.state.y);
        self.knowledge.observe_and_update(x, y, map);
        for dir in Direction::all().iter() {
            let (nx, ny) = movement::next_position(x, y, dir, map);
            if (nx, ny) != (x, y) {
                self.knowledge.observe_and_update(nx, ny, map);
            }
        }
    }

    fn try_move(
        &mut self,
        new_x: usize,
        new_y: usize,
        visited: &mut HashSet<(usize, usize)>,
        map: &Map,
    ) -> bool {
        if movement::is_valid_move(new_x, new_y, map)
            && !matches!(self.knowledge.get_tile(new_x, new_y), TileInfo::Obstacle)
        {
            self.state.x = new_x;
            self.state.y = new_y;
            visited.insert((new_x, new_y));
            self.state.use_energy(self.config.movement_energy_cost);
            true
        } else {
            false
        }
    }

    fn handle_returning_to_station(
        &mut self,
        sender: &Sender<RobotEvent>,
        map: &Arc<RwLock<Map>>,
        station_coords: (usize, usize),
        visited: &mut HashSet<(usize, usize)>,
    ) -> bool {
        let (station_x, station_y) = station_coords;
        if self.state.x == station_x && self.state.y == station_y {
            self.arrive_at_station(sender, visited);
            return true;
        }

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
        let (new_x, new_y) = movement::next_position(self.state.x, self.state.y, &direction, map_read);

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
        debug!(
            "Robot: {} Returning @ {:?}, Energy: {}",
            self.state.id,
            (self.state.x, self.state.y),
            self.state.energy
        );
        thread::sleep(config::random_sleep_duration(
            config::RETURN_SLEEP_MIN_MS,
            config::RETURN_SLEEP_MAX_MS,
        ));
        true
    }

    fn arrive_at_station(&mut self, sender: &Sender<RobotEvent>, visited: &mut HashSet<(usize, usize)>) {
        info!("Robot: {} Arrived station.", self.state.id);
        self.state.status = RobotStatus::AtStation;
        let k_clone = self.knowledge.clone();
        let ev = RobotEvent::ArrivedAtStation {
            id: self.state.id,
            knowledge: k_clone,
        };
        if let Err(e) = sender.send(ev) {
            error!("Robot: {} Failed send Arrived: {}", self.state.id, e);
            return;
        }
        info!("Robot: {} Waiting MergeComplete...", self.state.id);

        match self.merge_complete_receiver.recv_timeout(config::MERGE_TIMEOUT) {
            Ok(RobotEvent::MergeComplete { merged_knowledge, .. }) => {
                info!("Robot: {} MergeComplete OK.", self.state.id);
                self.knowledge = merged_knowledge;
                self.state.energy = self.state.max_energy;
                self.state.status = RobotStatus::Exploring;
                visited.clear();
                info!("Robot: {} Resuming exploration.", self.state.id);
            }
            Ok(o) => {
                warn!("Robot: {} Unexpected event: {:?}", self.state.id, o);
                self.state.status = RobotStatus::Exploring;
            }
            Err(RecvTimeoutError::Timeout) => {
                warn!("Robot: {} Merge Timeout.", self.state.id);
                self.state.status = RobotStatus::Exploring;
            }
            Err(RecvTimeoutError::Disconnected) => {
                error!("Robot: {} Merge channel disconnected.", self.state.id);
            }
        }
    }
}


// NOT WORKING - FIX TODO
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::robot::core::state::{RobotState, RobotStatus};
//     use crate::communication::channels::{RobotEvent, create_channel, ResourceType};
//     use crate::map::noise::Map;
//     use std::sync::{Arc, RwLock};
//     use std::thread;
//     use std::time::Duration;

//     #[test]
//     fn exploration_robot_finds_and_reports_resource_then_returns_to_station() {
//         let width = 7;
//         let height = 7;
//         let (tx, rx) = create_channel();
//         let mut map = Map::new(width, height, 42);
//         map.set_walkable(0, 0);
//         map.set_walkable(1, 0);
//         map.add_resource(1, 0, ResourceType::Minerals, 100);
//         let map = Arc::new(RwLock::new(map));
        
//         let robot_state = RobotState::new(1, 0, 0, RobotStatus::Exploring, 10);
//         let (_merge_tx, merge_rx) = create_channel();
//         let robot = ExplorationRobot::new(robot_state, width, height, merge_rx);

//         let tx_clone = tx.clone();
//         let map_clone = Arc::clone(&map);
//         thread::spawn(move || {
//             robot.start(tx_clone, map_clone);
//         });

//         let mut found_resource = false;
//         let mut returned_to_station = false;

//         let start = std::time::Instant::now();
//         while start.elapsed() < Duration::from_secs(10) {
//             if let Ok(event) = rx.recv_timeout(Duration::from_millis(200)) {
//                 match event {
//                     RobotEvent::ExplorationData { x, y, .. } if (x, y) == (1, 0) => {
//                         found_resource = true;
//                     }
//                     RobotEvent::ArrivedAtStation { id, .. } if id == 1 => {
//                         returned_to_station = true;
//                         let _ = tx.send(RobotEvent::MergeComplete {
//                             id,
//                             merged_knowledge: RobotKnowledge::new(width, height),
//                         });
//                     }
//                     RobotEvent::Shutdown { .. } => break,
//                     _ => {}
//                 }
//             }
//             if found_resource && returned_to_station {
//                 break;
//             }
//         }

//         assert!(found_resource, "Robot did not find the resource");
//         assert!(returned_to_station, "Robot did not return to station");
//     }
// }