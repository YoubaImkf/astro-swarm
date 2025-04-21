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
    
    #[test]
    fn test_station_merges_multiple_robots_knowledge() {
        let (tx, rx) = create_channel();
        let width = 20;
        let height = 20;
        let station = Station::new(tx.clone(), width, height);
    
        // Robot 1 discovers (0,0) top-left corner
        let mut knowledge1 = RobotKnowledge::new(width, height);
        knowledge1.update_tile(0, 0, TileInfo::Walkable);
        let event1 = RobotEvent::ArrivedAtStation { id: 1, knowledge: knowledge1 };
        station.process_event(&event1);
        let _ = rx.recv();
    
        // Robot 2 discovers (1,1)
        let mut knowledge2 = RobotKnowledge::new(width, height);
        knowledge2.update_tile(1, 1, TileInfo::Obstacle);
        let event2 = RobotEvent::ArrivedAtStation { id: 2, knowledge: knowledge2 };
        station.process_event(&event2);
        let received = rx.recv().expect("Should receive MergeComplete event");
    
        match received {
            RobotEvent::MergeComplete { merged_knowledge, .. } => {
                assert_eq!(merged_knowledge.get_tile(0, 0), &TileInfo::Walkable);
                assert_eq!(merged_knowledge.get_tile(1, 1), &TileInfo::Obstacle);
            }
            _ => panic!("Expected MergeComplete event"),
        }
    }

    #[test]
    fn test_station_handles_empty_knowledge() {
        let (tx, rx) = create_channel();
        let width = 4;
        let height = 4;
        let station = Station::new(tx, width, height);

        let knowledge = RobotKnowledge::new(width, height);
        let event = RobotEvent::ArrivedAtStation { id: 7, knowledge };
        station.process_event(&event);

        let received = rx.recv().expect("Should receive MergeComplete event");
        match received {
            RobotEvent::MergeComplete { merged_knowledge, .. } => {
                for x in 0..width {
                    for y in 0..height {
                        let tile = merged_knowledge.get_tile(x, y);
                        if tile == &TileInfo::Station {
                            continue; // Accept station tile
                        }
                        assert_eq!(tile, &TileInfo::Unknown);
                    }
                }
            }
            _ => panic!("Expected MergeComplete event"),
        }
    }

    #[test]
    fn test_station_merge_event_has_correct_id() {
        let (tx, rx) = create_channel();
        let station = Station::new(tx, 3, 3);

        let knowledge = RobotKnowledge::new(3, 3);
        let event = RobotEvent::ArrivedAtStation { id: 99, knowledge };
        station.process_event(&event);

        let received = rx.recv().expect("Should receive MergeComplete event");
        match received {
            RobotEvent::MergeComplete { id, .. } => assert_eq!(id, 99),
            _ => panic!("Expected MergeComplete event"),
        }
    }
}