use std::sync::{Arc, RwLock, mpsc};
use std::collections::HashMap;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::map::noise::Map;
use crate::robot::scientific::ScientificRobot;
use crate::robot::{RobotState, exploration::ExplorationRobot, collection::CollectionRobot};
use crate::communication::channels::{RobotEvent, ResourceType};

pub struct App {
    pub map: Arc<RwLock<Map>>,
    pub exploration_robots: Vec<RobotState>,
    pub collection_robots: Vec<RobotState>,
    pub scientific_robots: Vec<RobotState>, 

    pub event_receiver: mpsc::Receiver<RobotEvent>,
    event_sender: mpsc::Sender<RobotEvent>,

    pub collected_resources: HashMap<ResourceType, u32>,
    pub scientific_data: u32,
    pub total_explored: usize,
}

impl App {
    /// Creates the map using the given seeds,
    /// spawns resources and robots, and returns the App.
    pub fn new(width: usize, height: usize, map_seed: u32, resource_seed: u64) -> Self {
        let mut map = Map::new(width, height, map_seed);
        
        map.spawn_resources(width * height / 30, resource_seed);
        
        let (sender, receiver) = mpsc::channel();
        
        let map_arc = Arc::new(RwLock::new(map));
        
        // Create app
        let mut app = Self {
            map: map_arc.clone(),
            exploration_robots: Vec::new(),
            collection_robots: Vec::new(),
            scientific_robots: Vec::new(),
            event_receiver: receiver,
            event_sender: sender,
            collected_resources: HashMap::new(),
            total_explored: 0,
            scientific_data: 0,
        };
        
        app.spawn_robots(3, 2, 1, map_seed.into());
        
        app
    }
    
    fn spawn_robots(&mut self, exploration_count: usize, collection_count: usize,  scientific_count: usize, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        
        // Find walkable positions for robots
        let map_guard = self.map.read().unwrap();
        let mut walkable_positions = Vec::new();
        
        for y in 0..map_guard.height {
            for x in 0..map_guard.width {
                if !map_guard.is_obstacle(x, y) && !map_guard.has_resource(x, y) {
                    walkable_positions.push((x, y));
                }
            }
        }
        
        drop(map_guard);
        
        // Ensure we have enough positions
        if walkable_positions.len() < exploration_count + collection_count {
            panic!("Not enough walkable positions for robots");
        }
        
        // Shuffle positions
        walkable_positions.shuffle(&mut rng);
        
        // Create exploration robots
        for i in 0..exploration_count {
            let (x, y) = walkable_positions[i];
            let robot = ExplorationRobot::new(i as u32, x, y);
            
            // Store initial state
            self.exploration_robots.push(RobotState::new(i as u32, x, y));
            
            // Start robot thread
            let sender = self.event_sender.clone();
            let map_clone = self.map.clone();
            robot.start(sender, map_clone);
        }
        
        // Create collection robots
        for i in 0..collection_count {
            let (x, y) = walkable_positions[exploration_count + i];
            let mut robot = CollectionRobot::new((exploration_count + i) as u32, x, y);
            
            // Assign target resource type
            let resource_types = [
                ResourceType::Energy,
                ResourceType::Minerals,
                ResourceType::SciencePoints,
            ];
            
            if let Some(resource_type) = resource_types.choose(&mut rng) {
                robot.set_target_resource(resource_type.clone());
            }
            
            // Store initial state
            self.collection_robots.push(RobotState::new((exploration_count + i) as u32, x, y));
            
            // Start robot thread
            let sender = self.event_sender.clone();
            let map_clone = self.map.clone();
            robot.start(sender, map_clone);
        }  

                // Create scientific robots with specialized modules
        let scientific_modules = vec![
            ("Chemical Analyzer", 15, 2),
            ("Drill", 10, 3),
            ("High-Res Camera", 20, 1),
            ("Spectrometer", 25, 2),
            ("Sample Container", 5, 1),
        ];
        
        for i in 0..scientific_count {
            let (x, y) = walkable_positions[exploration_count + collection_count + i];
            let mut robot = ScientificRobot::new((exploration_count + collection_count + i) as u32, x, y);
            
            // Add 2-3 random modules to each scientific robot
            let module_count = rng.random_range(2..=3);
            let modules = scientific_modules.choose_multiple(&mut rng, module_count);
            
            for &(name, bonus, cost) in modules {
                robot.add_module(name, bonus, cost);
            }
            
            // Set target to science points
            robot.set_target_resource(ResourceType::SciencePoints);
            
            // Store initial state
            self.scientific_robots.push(RobotState::new((exploration_count + collection_count + i) as u32, x, y));
            
            // Start robot thread
            let sender = self.event_sender.clone();
            let map_clone = self.map.clone();
            robot.start(sender, map_clone);
        }
    }
    
    /// Update app state by processing events
    pub fn update(&mut self) {
        // Process all pending events
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                RobotEvent::ExplorationData { id, x, y, is_obstacle: _ } => {
                    // Update exploration robot position
                    if let Some(robot) = self.exploration_robots.iter_mut().find(|r| r.id == id) {
                        robot.x = x;
                        robot.y = y;
                    }
                    
                    self.total_explored += 1;
                },
                RobotEvent::CollectionData { id, x, y, resource_type, amount } => {
                    // Update collection robot position
                    if let Some(robot) = self.collection_robots.iter_mut().find(|r| r.id == id) {
                        robot.x = x;
                        robot.y = y;
                    }
                    
                    // Add collected resources
                    if let Some(resource_type) = resource_type {
                        *self.collected_resources.entry(resource_type).or_insert(0) += amount;
                    }
                },
                RobotEvent::ScienceData { id, x, y, resource_type, amount, modules } => {
                    // Update scientific robot position
                    if let Some(robot) = self.scientific_robots.iter_mut().find(|r| r.id == id) {
                        robot.x = x;
                        robot.y = y;
                    }
                    
                    // Add collected scientific data
                    self.scientific_data += amount;
                },
                RobotEvent::LowEnergy {id, remaining } => {
                },
                RobotEvent::ReturnToBase {id  } => {
                }
            }
        }
    }

    pub fn quit(&self) -> bool {
        false
    }
}