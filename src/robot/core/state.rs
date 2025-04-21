use crate::communication::channels::ResourceType;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RobotStatus {
    Idle,
    Exploring,
    Collecting,
    Analyzing,
    ReturningToStation,
    AtStation,
}

#[derive(Clone)]
pub struct RobotState {
    pub id: u32,
    pub x: usize,
    pub y: usize,
    pub energy: u32,
    pub max_energy: u32,
    pub collected_resources: HashMap<ResourceType, u32>,
    pub max_capacity: u32,
    pub status: RobotStatus,
}

impl RobotState {
    pub fn new(id: u32, start_x: usize, start_y: usize, initial_status: RobotStatus, max_energy: u32,) -> Self {
        Self {
            id,
            x: start_x,
            y: start_y,
            energy: max_energy,
            max_energy,
            collected_resources: HashMap::new(),
            max_capacity: 700,
            status: initial_status,
        }
    }

    pub fn use_energy(&mut self, amount: u32) -> bool {
        if self.energy >= amount {
            self.energy -= amount;
            true
        } else {
            self.energy = 0;
            false
        }
    }

    pub fn collect_resource(&mut self, resource_type: ResourceType, amount: u32) -> bool {
        let current_total: u32 = self.collected_resources.values().sum();

        if current_total + amount <= self.max_capacity {
            *self.collected_resources.entry(resource_type).or_insert(0) += amount;
            true
        } else {
            false
        }
    }

    pub fn is_full(&self) -> bool {
        let current_capacity = self.collected_resources.values().sum::<u32>();
        current_capacity >= self.max_capacity
    }

    pub fn needs_recharge(&self) -> bool {
        self.energy < 20
    }
}
