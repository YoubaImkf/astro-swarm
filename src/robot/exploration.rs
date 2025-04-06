use std::sync::{Arc, RwLock};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use rand::Rng;

use crate::communication::channels::RobotEvent;
use crate::map::noise::Map;
use super::movement;

pub struct ExplorationRobot {
    pub id: u32,
    pub x: usize,
    pub y: usize,
    pub energy: u32,
}

impl ExplorationRobot {
    pub fn new(id: u32, start_x: usize, start_y: usize) -> Self {
        Self { id, x: start_x, y: start_y, energy: 100 }
    }

    /// Starts the exploration loop
    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        thread::spawn(move || {
            let mut rng = rand::rng();
            
            loop {
                // Check energy levels
                if self.energy < 10 {
                    let event = RobotEvent::LowEnergy {
                        id: self.id,
                        remaining: self.energy,
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
                    thread::sleep(Duration::from_secs(2));
                    self.energy = 100;
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
                    self.energy -= 1; // Use energy for movement
                    
                    // Determine if this position has an obstacle
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