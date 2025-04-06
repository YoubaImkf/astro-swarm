use std::sync::{Arc, RwLock};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use rand::Rng;

use crate::communication::channels::{RobotEvent, ResourceType};
use crate::map::noise::Map;
use super::{RobotState, movement};

pub struct CollectionRobot {
    state: RobotState,
    target_resource: Option<ResourceType>,
}

impl CollectionRobot {
    pub fn new(id: u32, start_x: usize, start_y: usize) -> Self {
        Self {
            state: RobotState::new(id, start_x, start_y),
            target_resource: None,
        }
    }
    
    /// Set what type of resource this robot should prioritize collecting
    pub fn set_target_resource(&mut self, resource_type: ResourceType) {
        self.target_resource = Some(resource_type);
    }

    /// Start the collection robot in its own thread
    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        thread::spawn(move || {
            let mut rng = rand::rng();
            
            loop {
                // Check energy levels
                if self.state.energy < 15 {
                    let event = RobotEvent::LowEnergy {
                        id: self.state.id,
                        remaining: self.state.energy,
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                    
                    // Return to base if low on energy
                    let event = RobotEvent::ReturnToBase {
                        id: self.state.id,
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                    
                    // Simulate returning to base to recharge and offload resources
                    thread::sleep(Duration::from_secs(3));
                    self.state.energy = 100;
                    self.state.collected_resources.clear();
                    continue;
                }
                
                // Move in a random direction
                let direction = movement::Direction::random();
                let map_read = map.read().unwrap();
                let (new_x, new_y) = movement::next_position(
                    self.state.x, 
                    self.state.y, 
                    &direction, 
                    &map_read
                );
                
                // Check if move is valid
                if movement::is_valid_move(new_x, new_y, &map_read) {
                    self.state.x = new_x;
                    self.state.y = new_y;
                    self.state.use_energy(2); // Collection robots use more energy to move
                    
                    // Check if there's a resource at this position
                    if let Some((resource_type, amount)) = map_read.get_resource(new_x, new_y) {
                        // Only collect if we have capacity and either no target or matching target
                        if let Some(target) = &self.target_resource {
                            if target == &resource_type && self.state.collect_resource(resource_type.clone(), amount) {
                                let event = RobotEvent::CollectionData {
                                    id: self.state.id,
                                    x: new_x,
                                    y: new_y,
                                    resource_type: Some(resource_type),
                                    amount,
                                };
                                if sender.send(event).is_err() {
                                    break;
                                }
                            }
                        } else if self.state.collect_resource(resource_type.clone(), amount) {
                            let event = RobotEvent::CollectionData {
                                id: self.state.id,
                                x: new_x,
                                y: new_y,
                                resource_type: Some(resource_type),
                                amount,
                            };
                            if sender.send(event).is_err() {
                                break;
                            }
                        }
                    } else {
                        // Report that no resource was found
                        let event = RobotEvent::CollectionData {
                            id: self.state.id,
                            x: new_x,
                            y: new_y,
                            resource_type: None,
                            amount: 0,
                        };
                        if sender.send(event).is_err() {
                            break;
                        }
                    }
                }
                
                drop(map_read); // Release the lock
                
                // Sleep to simulate processing time
                thread::sleep(Duration::from_millis(rng.random_range(800..2000)));
            }
        });
    }
}