use std::sync::{Arc, RwLock};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use rand::Rng;

use crate::communication::channels::{RobotEvent, ResourceType};
use crate::map::noise::Map;
use super::{RobotState, movement};

pub struct Module {
    pub name: String,
    pub science_bonus: u32,
    pub energy_cost: u32,
}

pub struct ScientificRobot {
    state: RobotState,
    modules: Vec<Module>,
    target_resource: Option<ResourceType>,
}

impl ScientificRobot {
    pub fn new(id: u32, start_x: usize, start_y: usize) -> Self {
        Self {
            state: RobotState::new(id, start_x, start_y),
            modules: Vec::new(),
            target_resource: None,
        }
    }
    
    pub fn add_module(&mut self, name: &str, science_bonus: u32, energy_cost: u32) {
        self.modules.push(Module {
            name: name.to_string(),
            science_bonus,
            energy_cost,
        });
    }
    
    pub fn set_target_resource(&mut self, resource_type: ResourceType) {
        // Scientists primarily target SciencePoints
        if resource_type == ResourceType::SciencePoints {
            self.target_resource = Some(resource_type);
        }
    }
    
    pub fn analyze_science_point(&self, base_value: u32) -> u32 {
        let module_bonus: u32 = self.modules.iter()
            .map(|module| module.science_bonus)
            .sum();
        
        base_value + module_bonus
    }

    pub fn start(mut self, sender: Sender<RobotEvent>, map: Arc<RwLock<Map>>) {
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            
            loop {
                // Check energy levels
                if self.state.energy < 20 {
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
                    
                    // Simulate returning to base
                    thread::sleep(Duration::from_secs(3));
                    self.state.energy = 100;
                    self.state.collected_resources.clear();
                    continue;
                }
                
                // Move towards science points or explore randomly
                let direction = if let Some(target) = self.find_nearest_science_point(&map) {
                    self.move_towards_target(target, &map)
                } else {
                    movement::Direction::random()
                };
                
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
                    
                    // Scientists use more energy due to complex equipment
                    let module_energy_cost: u32 = self.modules.iter()
                        .map(|module| module.energy_cost)
                        .sum();
                    self.state.use_energy(1 + module_energy_cost);
                    
                    let resource_info = map_read.get_resource(new_x, new_y);
                    drop(map_read);
                    resource_info
                };
                
                // Handle resource collection (specifically science points)
                if let Some((resource_type, amount)) = resource_info {
                    if resource_type == ResourceType::SciencePoints {
                        // Scientists collect data but don't remove science points
                        let science_value = self.analyze_science_point(amount);
                        
                        if self.state.collect_resource(resource_type.clone(), science_value) {
                            // No need to remove science points as they're non-consumable
                            
                            // Send science data
                            let event = RobotEvent::ScienceData {
                                id: self.state.id,
                                x: self.state.x,
                                y: self.state.y,
                                resource_type: resource_type,
                                amount: science_value,
                                modules: self.modules.iter().map(|m| m.name.clone()).collect(),
                            };
                            if sender.send(event).is_err() {
                                break;
                            }
                        }
                    }
                }
                
                // Sleep to simulate processing time
                thread::sleep(Duration::from_millis(rng.random_range(1000..3000))); // Scientists work slower
            }
        });
    }
    
    fn find_nearest_science_point(&self, map: &Arc<RwLock<Map>>) -> Option<(usize, usize)> {
        let map_read = map.read().unwrap();
        let resources = map_read.get_all_resources();
        
        // Find science points
        let mut science_points = Vec::new();
        for ((x, y), resource) in resources {
            if resource.resource_type == crate::map::resources::ResourceType::SciencePoints {
                science_points.push((*x, *y));
            }
        }
        
        if science_points.is_empty() {
            return None;
        }
        
        // Find nearest one
        science_points.into_iter().min_by_key(|(x, y)| {
            let dx = *x as isize - self.state.x as isize;
            let dy = *y as isize - self.state.y as isize;
            (dx * dx + dy * dy) as usize  // Square distance
        })
    }
    
    fn move_towards_target(&self, target: (usize, usize), map: &Arc<RwLock<Map>>) -> movement::Direction {
        let (target_x, target_y) = target;
        
        // Determine best direction to move towards target
        if self.state.x < target_x {
            movement::Direction::Right
        } else if self.state.x > target_x {
            movement::Direction::Left
        } else if self.state.y < target_y {
            movement::Direction::Down
        } else {
            movement::Direction::Up
        }
    }
}