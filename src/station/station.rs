use log::info;

pub use crate::station::data_manager::DataManager;

use crate::communication::channels::RobotEvent;
use std::sync::{mpsc::Sender, Arc, RwLock};

pub struct Station {
    pub data_manager: Arc<RwLock<DataManager>>,
    event_sender: Sender<RobotEvent>,
}

impl Station {
    pub fn new(sender: Sender<RobotEvent>, width: usize, height: usize) -> Self {
        info!(
            "Initializing Station with DataManager for map size {}x{}",
            width, height
        );
        Self {
            data_manager: Arc::new(RwLock::new(DataManager::new(width, height))),
            event_sender: sender,
        }
    }

    pub fn process_event(&self, event: &RobotEvent) {
        match event {
            RobotEvent::ArrivedAtStation { id, knowledge } => {
                println!("Station: Robot {} arrived. Merging knowledge.", id);
                let merged_knowledge = {
                    let mut data_manager = self.data_manager.write().unwrap();
                    data_manager.merge_robot_knowledge(*id, knowledge);
                    data_manager.get_global_robot_knowledge()
                };

                let merge_event = RobotEvent::MergeComplete {
                    id: *id,
                    merged_knowledge,
                };
                if let Err(e) = self.event_sender.send(merge_event) {
                    eprintln!(
                        "Station Error: Failed to send MergeComplete to robot {}: {}",
                        id, e
                    );
                } else {
                    println!("Station: Sent MergeComplete to robot {}.", id);
                }
            }
            _ => {}
        }
    }

    pub fn update_simulation_map(&self, map: &Arc<RwLock<crate::map::noise::Map>>) {
        let data_manager = self.data_manager.read().unwrap();
        let mut map_guard = map.write().unwrap();
        data_manager.update_simulation_map(&mut map_guard);
    }
}
