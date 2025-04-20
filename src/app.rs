use std::{
    collections::HashMap,
    sync::{mpsc, Arc, RwLock},
};

use log::{error, info, warn};
use rand::{rngs::StdRng, seq::IndexedRandom, Rng, SeedableRng};

use crate::{
    communication::channels::{ResourceType, RobotEvent},
    map::noise::Map,
    robot::{
        collection::CollectionRobot, config::RECHARGE_ENERGY, exploration::ExplorationRobot,
        scientific::ScientificRobot, state::RobotStatus, RobotState,
    },
    station::station::Station,
};

pub struct App {
    pub map: Arc<RwLock<Map>>,
    pub exploration_robots: HashMap<u32, RobotState>,
    pub collection_robots: HashMap<u32, RobotState>,
    pub scientific_robots: HashMap<u32, RobotState>,
    pub event_receiver: mpsc::Receiver<RobotEvent>,
    event_sender: mpsc::Sender<RobotEvent>,
    robot_merge_senders: HashMap<u32, mpsc::Sender<RobotEvent>>,
    pub station: Station,
    pub collected_resources: HashMap<ResourceType, u32>,
    pub scientific_data: u64,
    pub total_explored: usize,
    pub map_width: usize,
    pub map_height: usize,
}

enum RobotType {
    Exploration,
    Collection,
    Scientific,
}

impl App {
    /// Creates a new `App` instance, initializing the map, station, and spawning initial robots.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the simulation map.
    /// * `height` - The height of the simulation map.
    /// * `map_seed` - Seed for generating the map layout (obstacles).
    /// * `resource_seed` - Seed for placing resources on the map.
    pub fn new(width: usize, height: usize, map_seed: u32, resource_seed: u64) -> Self {
        let mut map = Map::new(width, height, map_seed);
        
        map.spawn_resources(width * height / 30, resource_seed);

        let (main_sender, main_receiver) = mpsc::channel();
        let map_arc = Arc::new(RwLock::new(map));

        let station = Station::new(main_sender.clone(), width, height);

        let mut app = Self {
            map: map_arc,
            exploration_robots: HashMap::new(),
            collection_robots: HashMap::new(),
            scientific_robots: HashMap::new(),
            event_receiver: main_receiver,
            event_sender: main_sender,
            robot_merge_senders: HashMap::new(),
            station,
            collected_resources: HashMap::new(),
            scientific_data: 0,
            total_explored: 0,
            map_width: width,
            map_height: height,
        };

        app.spawn_robots(1, 1, 1, map_seed.into());
        app
    }

    /// Spawns the specified number of each robot type at random valid locations.
    fn spawn_robots(
        &mut self,
        exploration_count: usize,
        collection_count: usize,
        scientific_count: usize,
        seed: u64,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);
        let walkable_positions = self.find_walkable_spawn_positions();

        let total_robots_to_spawn = exploration_count + collection_count + scientific_count;
        if walkable_positions.len() < total_robots_to_spawn {
            error!(
                "Insufficient walkable spawn positions (need {}, found {}). Cannot spawn all requested robots.",
                total_robots_to_spawn,
                walkable_positions.len()
            );
            panic!("Robot spawn failed: Insufficient space.");
        }

        let mut available_positions: Vec<(usize, usize)> = walkable_positions
            .choose_multiple(&mut rng, total_robots_to_spawn)
            .cloned()
            .collect();

        let mut current_id_counter = 0;

        // Spawn Exploration Robots
        for _ in 0..exploration_count {
            if let Some(pos) = available_positions.pop() {
                self.spawn_robot_instance(
                    &mut current_id_counter,
                    pos,
                    RobotType::Exploration,
                    &mut rng,
                );
            }
        }

        // Spawn Collection Robots
        for _ in 0..collection_count {
            if let Some(pos) = available_positions.pop() {
                self.spawn_robot_instance(
                    &mut current_id_counter,
                    pos,
                    RobotType::Collection,
                    &mut rng,
                );
            }
        }

        // Spawn Scientific Robots
        for _ in 0..scientific_count {
            if let Some(pos) = available_positions.pop() {
                self.spawn_robot_instance(
                    &mut current_id_counter,
                    pos,
                    RobotType::Scientific,
                    &mut rng,
                );
            }
        }
    }

    fn find_walkable_spawn_positions(&self) -> Vec<(usize, usize)> {
        let map_guard = self
            .map
            .read()
            .expect("Map lock poisoned during spawn pos search");
        let (station_x, station_y) = (map_guard.width / 2, map_guard.height / 2);
        let mut positions = Vec::new();

        for y in 0..map_guard.height {
            for x in 0..map_guard.width {
                if !map_guard.is_obstacle(x, y)
                    && !map_guard.has_resource(x, y)
                    && !(x == station_x && y == station_y)
                {
                    positions.push((x, y));
                }
            }
        }
        positions
    }

    fn spawn_robot_instance(
        &mut self,
        current_id_counter: &mut u32,
        position: (usize, usize),
        robot_type: RobotType,
        rng: &mut StdRng,
    ) {
        let id = *current_id_counter;
        let (x, y) = position;

        // Create dedicatedd channel for MergeComplete event for thi robot
        let (merge_sender, merge_receiver) = mpsc::channel();
        self.robot_merge_senders.insert(id, merge_sender);

        let map_clone = self.map.clone();
        let event_sender_clone = self.event_sender.clone();

        match robot_type {
            RobotType::Exploration => {
                let robot_state = RobotState::new(id, x, y, RobotStatus::Exploring);
                let robot_logic = ExplorationRobot::new(
                    robot_state.clone(),
                    self.map_width,
                    self.map_height,
                    merge_receiver,
                );
                self.exploration_robots.insert(id, robot_state);
                robot_logic.start(event_sender_clone, map_clone);
                info!("Spawned Exploration Robot {}", id);
            }
            RobotType::Collection => {
                let robot_state = RobotState::new(id, x, y, RobotStatus::Collecting);
                let mut robot_logic = CollectionRobot::new(
                    robot_state.clone(),
                    self.map_width,
                    self.map_height,
                    merge_receiver,
                );

                // Assign target resource type
                let resource_types = [ResourceType::Energy, ResourceType::Minerals];
                if let Some(target) = resource_types.choose(rng) {
                    robot_logic.set_target_resource(target.clone());
                }
                self.collection_robots.insert(id, robot_state);
                robot_logic.start(event_sender_clone, map_clone);
                info!("Spawned Collection Robot {}", id);
            }
            RobotType::Scientific => {
                let robot_state = RobotState::new(id, x, y, RobotStatus::Analyzing);
                let mut robot_logic = ScientificRobot::new(
                    robot_state.clone(),
                    self.map_width,
                    self.map_height,
                    merge_receiver,
                );

                // Assign modules
                let scientific_modules = vec![
                    ("Chemical Analyzer", 15, 2),
                    ("Drill", 10, 3),
                    ("High-Res Camera", 20, 1),
                    ("Spectrometer", 25, 2),
                    ("Sample Container", 5, 1),
                ];
                if !scientific_modules.is_empty() {
                    let module_count = rng.random_range(1..=scientific_modules.len().min(3));
                    for &(name, bonus, cost) in
                        scientific_modules.choose_multiple(rng, module_count)
                    {
                        robot_logic.add_module(name, bonus, cost);
                    }
                }
                self.scientific_robots.insert(id, robot_state);
                robot_logic.start(event_sender_clone, map_clone);
                info!("Spawned Scientific Robot {}", id);
            }
        }
        *current_id_counter += 1;
    }

    pub fn update(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
            if matches!(event, RobotEvent::ArrivedAtStation { .. }) {
                self.station.process_event(&event);
            }

            match event {
                RobotEvent::ExplorationData { id, x, y, .. } => {
                    if let Some(robot) = self.get_robot_state_mut(id) {
                        robot.x = x;
                        robot.y = y;
                    }
                }
                RobotEvent::CollectionData {
                    id,
                    x,
                    y,
                    resource_type,
                    amount,
                } => {
                    if let Some(robot) = self.get_robot_state_mut(id) {
                        robot.x = x;
                        robot.y = y;
                    }

                    if let Some(res_type) = resource_type {
                        if amount > 0 {
                            *self.collected_resources.entry(res_type).or_insert(0) += amount;
                        }
                    }
                }
                RobotEvent::ScienceData {
                    id, x, y, amount, ..
                } => {
                    if let Some(robot) = self.get_robot_state_mut(id) {
                        robot.x = x;
                        robot.y = y;
                    }

                    self.scientific_data += amount as u64;
                }
                RobotEvent::LowEnergy { id, remaining } => {
                    if let Some(robot) = self.get_robot_state_mut(id) {
                        robot.energy = remaining;
                    } else {
                        warn!("Received LowEnergy event for unknown robot ID: {}", id);
                    }
                }
                RobotEvent::MergeComplete { id, .. } => {
                    let robot_type = if self.exploration_robots.contains_key(&id) {
                        Some(RobotType::Exploration)
                    } else if self.collection_robots.contains_key(&id) {
                        Some(RobotType::Collection)
                    } else if self.scientific_robots.contains_key(&id) {
                        Some(RobotType::Scientific)
                    } else {
                        None
                    };

                    if let Some(robot) = self.get_robot_state_mut(id) {
                        robot.energy = RECHARGE_ENERGY;
                        robot.collected_resources.clear();

                        match robot_type {
                            Some(RobotType::Exploration) => {
                                robot.status = RobotStatus::Exploring;
                            }
                            Some(RobotType::Collection) => {
                                robot.status = RobotStatus::Collecting;
                            }
                            Some(RobotType::Scientific) => {
                                robot.status = RobotStatus::Analyzing;
                            }
                            None => {
                                warn!("Robot type not found for ID: {}", id);
                            }
                        }
                    } else {
                        warn!("Received MergeComplete event for unknown robot ID: {}", id);
                    }
                }
                RobotEvent::ArrivedAtStation { id, .. } => {
                    if let Some(robot) = self.get_robot_state_mut(id) {
                        robot.status = RobotStatus::AtStation;
                    }
                }
                RobotEvent::Shutdown { id, reason } => {
                    info!("Robot {} shutting down: {}", id, reason);

                    self.exploration_robots.remove(&id);
                    self.collection_robots.remove(&id);
                    self.scientific_robots.remove(&id);
                    self.robot_merge_senders.remove(&id);
                }
                RobotEvent::ReturnToBase { id } => {
                    if let Some(robot) = self.get_robot_state_mut(id) {
                        robot.status = RobotStatus::ReturningToStation;
                    } else {
                        warn!("Received ReturnToBase event for unknown robot ID: {}", id);
                    }
                }
            }
        }
    }

    /// Gets a mutable reference to a robot's state regardless of its type.
    fn get_robot_state_mut(&mut self, robot_id: u32) -> Option<&mut RobotState> {
        if let Some(robot) = self.exploration_robots.get_mut(&robot_id) {
            Some(robot)
        } else if let Some(robot) = self.collection_robots.get_mut(&robot_id) {
            Some(robot)
        } else {
            self.scientific_robots.get_mut(&robot_id)
        }
    }
}
