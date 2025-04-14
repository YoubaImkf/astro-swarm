use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone)]
pub enum RobotEvent {
    /// Sent when a robot explores a new tile
    ExplorationData {
        id: u32,
        x: usize,
        y: usize,
        is_obstacle: bool,
    },
    
    /// Sent when a robot attempts to collect a resource
    CollectionData {
        id: u32,
        x: usize,
        y: usize,
        resource_type: Option<ResourceType>,
        amount: u32,
    },
    
    /// Sent when a robot's energy is low
    LowEnergy {
        id: u32,
        remaining: u32,
    },
    
    /// Sent when a robot returns to base
    ReturnToBase {
        id: u32,
    },

    ScienceData {
        id: u32,
        x: usize,
        y: usize,
        resource_type: ResourceType,
        amount: u32,
        modules: Vec<String>,
    }
}

/// Types of resources robots can collect
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Energy,
    Minerals,
    SciencePoints,
}

/// Creates a new communication channel for robot-station communication
pub fn create_channel() -> (Sender<RobotEvent>, Receiver<RobotEvent>) {
    channel()
}