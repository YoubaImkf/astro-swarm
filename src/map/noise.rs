use noise::{NoiseFn, Perlin};
use rand::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::fmt;

use super::resources::{Resource, ResourceManager, ResourceType};

pub struct Map {
    pub width: usize,
    pub height: usize,
    pub station_area: Vec<(usize, usize)>,
    data: Vec<Vec<bool>>, // true = obstacle (#), false = walkable (.)
    resource_manager: ResourceManager,
}

impl Map {
    /// Creates a new `Map` using Perlin noise
    ///
    /// # Parameters
    /// - `width`: The width of the map
    /// - `height`: The height of the map
    /// - `seed`: A seed for noise generation to make it reproductible
    ///
    /// # Returns
    /// A new `Map` instance with obstacles generated based on Perlin noise
    pub fn new(width: usize, height: usize, seed: u32) -> Self {
        let perlin = Perlin::new(seed);

        let data = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| perlin.get([x as f64 / 10.0, y as f64 / 10.0]) > 0.0)
                    .collect()
            })
            .collect();

        let cx = width / 2;
        let cy = height / 2;
        let mut station_area = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                let x = (cx as isize + dx) as usize;
                let y = (cy as isize + dy) as usize;
                station_area.push((x, y));
            }
        }

        let mut map = Self {
            width,
            height,
            data,
            station_area,
            resource_manager: ResourceManager::new(),
        };

        // Ensure station is walkable
        for &(x, y) in &map.station_area {
            map.data[y][x] = false;
        }

        map.connect_isolated_regions();
        map
    }

    /// Spawns resources at random walkable positions
    pub fn spawn_resources(&mut self, count: usize, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let walkable_positions: Vec<_> = self
            .data
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, &cell)| if !cell { Some((x, y)) } else { None })
            })
            .collect();
        let resource_types = [
            ResourceType::Energy,
            ResourceType::Minerals,
            ResourceType::SciencePoints,
        ];
        for &(x, y) in walkable_positions.choose_multiple(&mut rng, count) {
            let resource_type = resource_types.choose(&mut rng).unwrap().clone();
            
            let resource_amount = match resource_type {
                ResourceType::SciencePoints => rng.random_range(1..=5),
                _ => rng.random_range(10..100),
            };

            self.resource_manager
                .add_resource(x, y, resource_type, resource_amount);
        }
    }

    /// Ensures all walkable areas are connected
    fn connect_isolated_regions(&mut self) {
        let mut visited = vec![vec![false; self.width]; self.height];
        let mut regions = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if !self.data[y][x] && !visited[y][x] {
                    regions.push(self.collect_connected_walkable_cells(x, y, &mut visited));
                }
            }
        }

        if regions.len() <= 1 {
            return;
        }
        let main_region = &regions[0];
        for region in regions.iter().skip(1) {
            let (main_x, main_y) = main_region[0];
            let (other_x, other_y) = region[0];
            self.create_path(main_x, main_y, other_x, other_y);
        }
    }

    /// Find all connected walkable regions from a starting point
    ///
    /// # Parameters
    /// - `start_x`, `start_y`: Coordinates to begin the flood fill
    /// - `visited`: A mutable reference to track visited cells
    ///
    /// # Returns
    /// A vector of `(x, y)` coordinates belonging to the connected region
    fn collect_connected_walkable_cells(
        &self,
        start_x: usize,
        start_y: usize,
        visited: &mut Vec<Vec<bool>>,
    ) -> Vec<(usize, usize)> {
        let mut region = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back((start_x, start_y));
        visited[start_y][start_x] = true;

        while let Some((x, y)) = queue.pop_front() {
            region.push((x, y));
            for (nx, ny) in Self::valid_neighbors(x, y, self.width, self.height) {
                if !visited[ny][nx] && !self.data[ny][nx] {
                    visited[ny][nx] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        region
    }

    /// Returns an iterator over valid neighboring cells (left, right, up, down)
    ///
    /// # Parameters
    /// - `x`, `y`: The current cell position
    /// - `width`, `height`: The map dimensions
    ///
    /// # Returns
    /// An iterator yielding valid `(x, y)` neighbor coordinates
    pub fn valid_neighbors(
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = (usize, usize)> {
        let directions = [(-1, 0), (1, 0), (0, -1), (0, 1)];

        directions.into_iter().filter_map(move |(dx, dy)| {
            let new_x = x as isize + dx;
            let new_y = y as isize + dy;
            if new_x >= 0 && new_x < width as isize && new_y >= 0 && new_y < height as isize {
                Some((new_x as usize, new_y as usize))
            } else {
                None
            }
        })
    }

    /// Creates a path between two points in the map.
    ///
    /// The path is carved in a simple "L" shape, moving horizontally first, then vertically.
    fn create_path(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let (mut x, mut y) = (x1 as isize, y1 as isize);
        let (target_x, target_y) = (x2 as isize, y2 as isize);

        // Carve the horizontal segment of the path.
        while x != target_x {
            self.data[y as usize][x as usize] = false;
            x += if x < target_x { 1 } else { -1 };
        }
        // Carve the vertical segment of the path
        while y != target_y {
            self.data[y as usize][x as usize] = false;
            y += if y < target_y { 1 } else { -1 };
        }
    }

    pub fn get_resource(
        &self,
        x: usize,
        y: usize,
    ) -> Option<(crate::communication::channels::ResourceType, u32)> {
        self.resource_manager.get_resource(x, y).map(|ressource| {
            let channel_resource_type = match ressource.resource_type {
                ResourceType::Energy => crate::communication::channels::ResourceType::Energy,
                ResourceType::Minerals => crate::communication::channels::ResourceType::Minerals,
                ResourceType::SciencePoints => {
                    crate::communication::channels::ResourceType::SciencePoints
                }
            };
            (channel_resource_type, ressource.amount)
        })
    }

    /// Removes a resource at the given coordinates if it's consumable (Energy, Minerals)
    /// For SciencePoints (non-consumable), just returns the resource info without removing it
    ///
    /// # Parameters
    /// - `x`, `y`: The coordinates to get/remove a resource from
    ///
    /// # Returns
    /// Some((resource_type, amount)) if a resource was found, None if no resource existed
    pub fn remove_resource(
        &mut self,
        x: usize,
        y: usize,
    ) -> Option<(crate::communication::channels::ResourceType, u32)> {
        let (r_type, amount) = {
            let resource = self.resource_manager.get_resource(x, y)?;
            (resource.resource_type.clone(), resource.amount)
        };

        let is_consumable = matches!(r_type, ResourceType::Energy | ResourceType::Minerals);
        let channel_resource_type = match r_type {
            ResourceType::Energy => crate::communication::channels::ResourceType::Energy,
            ResourceType::Minerals => crate::communication::channels::ResourceType::Minerals,
            ResourceType::SciencePoints => {
                crate::communication::channels::ResourceType::SciencePoints
            }
        };

        if is_consumable {
            self.resource_manager.remove_resource(x, y);
        }

        Some((channel_resource_type, amount))
    }

    pub fn add_resource(
        &mut self,
        x: usize,
        y: usize,
        resource_type: crate::communication::channels::ResourceType,
        amount: u32,
    ) {
        use super::resources::ResourceType as InternalResourceType;
        let internal_type = match resource_type {
            crate::communication::channels::ResourceType::Energy => InternalResourceType::Energy,
            crate::communication::channels::ResourceType::Minerals => InternalResourceType::Minerals,
            crate::communication::channels::ResourceType::SciencePoints => InternalResourceType::SciencePoints,
        };
        self.resource_manager.add_resource(x, y, internal_type, amount);
    }

    pub fn set_walkable(&mut self, x: usize, y: usize) {
        if let Some(row) = self.data.get_mut(y) {
            if let Some(cell) = row.get_mut(x) {
                *cell = false;
            }
        }
    }

    pub fn get_all_resources(&self) -> &HashMap<(usize, usize), Resource> {
        self.resource_manager.get_all_resources()
    }

    pub fn has_resource(&self, x: usize, y: usize) -> bool {
        self.resource_manager.has_resource(x, y)
    }

    pub fn is_obstacle(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return true; // Out of bounds is considered an obstacle
        }
        self.data[y][x]
    }

    pub fn is_station(&self, x: usize, y: usize) -> bool {
        self.station_area.contains(&(x, y))
    }
}

// Formats the `Map` as a grid of characters (`#` for obstacles, `.` for walkable tiles)
impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resources = self.resource_manager.get_all_resources();

        for y in 0..self.height {
            for x in 0..self.width {
                let symbol = if self.is_station(x, y) {
                    '⌂'
                } else if self.data[y][x] {
                    '█'
                } else if let Some(resource) = resources.get(&(x, y)) {
                    match resource.resource_type {
                        ResourceType::Energy => 'E',        // ⚡
                        ResourceType::Minerals => 'M',      // ⛏
                        ResourceType::SciencePoints => 'S', // 🧪
                    }
                } else {
                    ' '
                };
                write!(f, "{symbol}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
