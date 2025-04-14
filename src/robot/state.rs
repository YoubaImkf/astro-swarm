use std::collections::HashMap;
use crate::communication::channels::ResourceType;

pub struct RobotState {
    pub id: u32,
    pub x: usize,
    pub y: usize,
    pub energy: u32,
    pub collected_resources: HashMap<ResourceType, u32>,
    pub capacity: u32,
}

impl RobotState {
    pub fn new(id: u32, start_x: usize, start_y: usize) -> Self {
        Self {
            id,
            x: start_x,
            y: start_y,
            energy: 100,
            collected_resources: HashMap::new(),
            capacity: 50, // Default capacity
        }
    }
    
    pub fn use_energy(&mut self, amount: u32) -> bool {
        if self.energy >= amount {
            self.energy -= amount;
            true
        } else {
            false
        }
    }
    
    pub fn collect_resource(&mut self, resource_type: ResourceType, amount: u32) -> bool {
        // Calculate current total of collected resources
        let current_total: u32 = self.collected_resources.values().sum();
        
        if current_total + amount <= self.capacity {
            *self.collected_resources.entry(resource_type).or_insert(0) += amount;
            true
        } else {
            false
        }
    }
}