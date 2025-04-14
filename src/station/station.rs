pub use crate::station::data_manager::DataManager;

use std::sync::{Arc, RwLock};
use crate::map::noise::Map;
use crate::communication::channels::RobotEvent;

pub struct Station {
    pub data_manager: Arc<RwLock<DataManager>>,
    pub map: Arc<RwLock<Map>>,
}

impl Station {
    pub fn new(map: Arc<RwLock<Map>>) -> Self {
        Self {
            data_manager: Arc::new(RwLock::new(DataManager::new())),
            map,
        }
    }
    
    pub fn process_event(&self, event: RobotEvent) {
        let mut data_manager = self.data_manager.write().unwrap();
        
        match event {
            RobotEvent::ExplorationData { id, x, y, is_obstacle } => {
                data_manager.add_exploration_data(x, y, is_obstacle);
            },
            RobotEvent::CollectionData { id, x, y, resource_type, amount } => {
                if let Some(resource_type) = resource_type {
                    data_manager.add_resource_data(x, y, resource_type, amount, id);
                }
            },
            RobotEvent::ScienceData { id, x, y, resource_type, amount, .. } => {
                data_manager.add_science_data(x, y, amount, id);
                data_manager.add_resource_data(x, y, resource_type, amount, id);
            },
            _ => {},
        }
    }
    
    // Resolve conflicts and update the shared map
    pub fn update_map(&self) {
        let data_manager = self.data_manager.read().unwrap();
        let resource_data = data_manager.resolve_conflicts();
        let exploration_data = data_manager.get_exploration_map();
        
        // Update map with resolved data
        // (This would require extending the Map interface)
    }
}  