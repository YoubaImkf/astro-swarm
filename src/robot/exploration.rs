use std::sync::{Arc, RwLock};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use rand::Rng;

use crate::communication::channels::RobotEvent;
use crate::map::noise::Map;
use super::{movement, RobotState};
use super::knowledge::RobotKnowledge;

pub struct ExplorationRobot {
    pub id: u32,
    pub x: usize,
    pub y: usize,
    state: RobotState,
    pub knowledge: RobotKnowledge,
}

impl ExplorationRobot {
    pub fn new(id: u32, start_x: usize, start_y: usize,map_width: usize, map_height: usize) -> Self {
        Self { 
            id, 
            x: start_x, 
            y: start_y, 
            state: RobotState::new(id, start_x, start_y), 
            knowledge: RobotKnowledge::new(map_width, map_height),
        }
    }

    fn update_knowledge(&mut self, map: &Map) {
        self.knowledge.observe_and_update(self.x, self.y, map);
    }

    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        thread::spawn(move || {
            let mut rng = rand::rng();
            
            loop {
                // Check energy levels
                if self.state.energy < 10 {
                    let event = RobotEvent::LowEnergy {
                        id: self.id,
                        remaining: self.state.energy,
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                    
                    // Return to base to recharge
                    let event = RobotEvent::ReturnToBase {
                        id: self.id,
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                    
                    // Simulate recharging
                    thread::sleep(Duration::from_millis(800));
                    self.state.energy = 100;
                    continue;
                }
                
                // Move in a random direction
                let direction = movement::Direction::random();
                let map_read = map.read().unwrap();
                let (new_x, new_y) = movement::next_position(
                    self.x, 
                    self.y, 
                    &direction, 
                    &map_read
                );
                
                // Check if move is valid
                if movement::is_valid_move(new_x, new_y, &map_read) {
                    self.x = new_x;
                    self.y = new_y;
                    self.state.use_energy(1); // Use energy for movement
                    
                    // Update the robot local knowledge after a move
                    self.update_knowledge(&map_read);

                    let is_obstacle = map_read.is_obstacle(new_x, new_y);
                    
                    // Send exploration data
                    let event = RobotEvent::ExplorationData {
                        id: self.id,
                        x: new_x,
                        y: new_y,
                        is_obstacle,
                    };
                    
                    if sender.send(event).is_err() {
                        break;
                    }
                }
                
                drop(map_read); // Release the lock
                
                // Sleep to simulate processing time
                thread::sleep(Duration::from_millis(rng.random_range(500..1000)));
            }
        });
    }
}