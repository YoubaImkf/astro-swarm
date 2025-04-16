use std::sync::mpsc::{channel, Receiver, Sender};
use crate::robot::knowledge::RobotKnowledge;

#[derive(Debug, Clone)]
pub enum RobotEvent {
    ExplorationData {
        id: u32,
        x: usize,
        y: usize,
        is_obstacle: bool,
    },
    CollectionData {
        id: u32,
        x: usize,
        y: usize,
        resource_type: Option<ResourceType>,
        amount: u32,
    },
    ScienceData {
        id: u32,
        x: usize,
        y: usize,
        resource_type: ResourceType,
        amount: u32,
        modules: Vec<String>,
    },
    LowEnergy {
        id: u32,
        remaining: u32,
    },
    ReturnToBase {
        id: u32,
    },
    ArrivedAtStation {
        id: u32,
        knowledge: RobotKnowledge,
    },
    MergeComplete {
        id: u32,
        merged_knowledge: RobotKnowledge,
    },
    Shutdown {
        id: u32,
        reason: String,
    },
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