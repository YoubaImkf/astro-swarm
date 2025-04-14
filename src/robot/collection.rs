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
    
    pub fn set_target_resource(&mut self, resource_type: ResourceType) {
        self.target_resource = Some(resource_type);
    }

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
                    
                    self.state.energy = 100;
                    self.state.collected_resources.clear();
                    continue;
                }
                
                let direction = movement::Direction::random();

                let resource_info = {
                    let map_read = map.read().unwrap();
                    let (new_x, new_y) = movement::next_position(
                        self.state.x, 
                        self.state.y, 
                        &direction, 
                        &map_read
                    );
                    
                    if !movement::is_valid_move(new_x, new_y, &map_read) {
                        drop(map_read);
                        continue;
                    }
                    
                    self.state.x = new_x;
                    self.state.y = new_y;
                    self.state.use_energy(2); // Collection robots use more energy to move
                    
                    let resource_info = map_read.get_resource(new_x, new_y);
                    drop(map_read);
                    resource_info
                };
                
                // Handle resource collection if found
                if let Some((resource_type, amount)) = resource_info {
                    let should_collect = self.target_resource.is_none() || 
                                        self.target_resource.as_ref() == Some(&resource_type);
                    
                    if should_collect && self.state.collect_resource(resource_type.clone(), amount) {
                        let mut map_write = map.write().unwrap();
                        map_write.remove_resource(self.state.x, self.state.y);
                        drop(map_write);
                        
                        let event = RobotEvent::CollectionData {
                            id: self.state.id,
                            x: self.state.x,
                            y: self.state.y,
                            resource_type: Some(resource_type),
                            amount,
                        };
                        if sender.send(event).is_err() {
                            break;
                        }
                    } else {
                        let event = RobotEvent::CollectionData {
                            id: self.state.id,
                            x: self.state.x,
                            y: self.state.y,
                            resource_type: Some(resource_type),
                            amount: 0, // Zero amount indicates not collected
                        };
                        if sender.send(event).is_err() {
                            break;
                        }
                    }
                } else {
                    // No resource found, just report movement
                    let event = RobotEvent::CollectionData {
                        id: self.state.id,
                        x: self.state.x,
                        y: self.state.y,
                        resource_type: None,
                        amount: 0,
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                }
                
                // Sleep to simulate processing time
                thread::sleep(Duration::from_millis(rng.random_range(500..1000)));
            }
        });
    }
}