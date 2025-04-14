use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::communication::channels::ResourceType;

// Struct to track resource data with versioning
#[derive(Clone, Debug)]
pub struct ResourceVersion {
    pub amount: u32,
    pub timestamp: DateTime<Utc>,
    pub robot_id: u32,
}

// Data manager that resolves conflicts when multiple robots report on same area
pub struct DataManager {
    resource_data: HashMap<(usize, usize), Vec<ResourceVersion>>,
    exploration_data: HashMap<(usize, usize), bool>,
    science_data: HashMap<(usize, usize), Vec<ResourceVersion>>,
}

impl DataManager {
    pub fn new() -> Self {
        Self {
            resource_data: HashMap::new(),
            exploration_data: HashMap::new(),
            science_data: HashMap::new(),
        }
    }
    
    // Add exploration data with conflict resolution
    pub fn add_exploration_data(&mut self, x: usize, y: usize, is_obstacle: bool) {
        self.exploration_data.insert((x, y), is_obstacle);
    }
    
    // Add resource data with Git-like versioning
    pub fn add_resource_data(&mut self, x: usize, y: usize, resource_type: ResourceType, 
                            amount: u32, robot_id: u32) {
        let version = ResourceVersion {
            amount,
            timestamp: Utc::now(),
            robot_id,
        };
        
        self.resource_data
            .entry((x, y))
            .or_insert_with(Vec::new)
            .push(version);
    }
    
    // Add science data with versioning
    pub fn add_science_data(&mut self, x: usize, y: usize, amount: u32, robot_id: u32) {
        let version = ResourceVersion {
            amount,
            timestamp: Utc::now(),
            robot_id,
        };
        
        self.science_data
            .entry((x, y))
            .or_insert_with(Vec::new)
            .push(version);
    }
    
    // Resolve conflicts by taking the latest version for each location
    pub fn resolve_conflicts(&self) -> HashMap<(usize, usize), ResourceVersion> {
        let mut resolved = HashMap::new();
        
        for (&coords, versions) in &self.resource_data {
            if let Some(latest) = versions.iter()
                .max_by_key(|v| v.timestamp) {
                resolved.insert(coords, latest.clone());
            }
        }
        
        resolved
    }
    
    // Merge conflicting science data by combining the findings
    pub fn merge_science_data(&self) -> HashMap<(usize, usize), u32> {
        let mut merged = HashMap::new();
        
        for (&coords, versions) in &self.science_data {
            let total: u32 = versions.iter().map(|v| v.amount).sum();
            merged.insert(coords, total);
        }
        
        merged
    }
    
    // Get the exploration map
    pub fn get_exploration_map(&self) -> &HashMap<(usize, usize), bool> {
        &self.exploration_data
    }
}