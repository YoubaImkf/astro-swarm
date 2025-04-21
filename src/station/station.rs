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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::robot::core::knowledge::{RobotKnowledge, TileInfo};
    use crate::communication::channels::{RobotEvent, create_channel};

    #[test]
    fn test_station_merges_knowledge_and_sends_merge_complete() {
        let (tx, rx) = create_channel();
        let width = 10;
        let height = 10;
        let station = Station::new(tx.clone(), width, height);

        // Create robot knowledge with a known tile
        let mut knowledge = RobotKnowledge::new(width, height);
        knowledge.update_tile(1, 1, TileInfo::Walkable);

        // Simulate robot arrival
        let event = RobotEvent::ArrivedAtStation {
            id: 42,
            knowledge: knowledge.clone(),
        };
        station.process_event(&event);

        // Check that the MergeComplete was sent successfluy
        let received = rx.recv().expect("Should receive MergeComplete event");
        match received {
            RobotEvent::MergeComplete { id, merged_knowledge } => {
                assert_eq!(id, 42);
                // The merged knowledge should contain the updated tile ! 
                assert_eq!(merged_knowledge.get_tile(1, 1), &TileInfo::Walkable);
            }
            _ => panic!("Expected MergeComplete event"),
        }
    }

    #[test]
    fn test_station_handles_unknown_event_gracefully() {
        let (tx, rx) = create_channel();
        let station = Station::new(tx, 5, 5);

        // Send any event
        let event = RobotEvent::Shutdown {
            id: 1,
            reason: "test".to_string(),
        };

        // Should not throw an error
        station.process_event(&event);
        assert!(rx.try_recv().is_err());
    }
}