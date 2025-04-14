use noise::{NoiseFn, Perlin};
use rand::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::fmt;

use super::resources::{Resource, ResourceManager, ResourceType};

pub struct Map {
    pub width: usize,
    pub height: usize,
    data: Vec<Vec<bool>>, // true = obstacle (#), false = walkable (.)
    resource_manager: ResourceManager, // Stores resource positions
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

        let data: Vec<Vec<bool>> = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| perlin.get([x as f64 / 10.0, y as f64 / 10.0]) > 0.0)
                    .collect()
            })
            .collect();

        let mut map = Self {
            width,
            height,
            data,
            resource_manager: ResourceManager::new(),
        };
        map.connect_isolated_regions();
        map
    }

    /// Spawns a given number of resources at random walkable positions
    pub fn spawn_resources(&mut self, count: usize, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let walkable_positions: Vec<(usize, usize)> = self.data.iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter().enumerate()
                    .filter_map(move |(x, &cell)| if !cell { Some((x, y)) } else { None })
            })
            .collect();

        let resource_types = [
            ResourceType::Energy,
            ResourceType::Minerals,
            ResourceType::SciencePoints,
        ];

        walkable_positions.choose_multiple(&mut rng, count)
            .for_each(|&(x, y)| {
                let resource_type = resource_types.choose(&mut rng).unwrap();
                let amount = rng.random_range(10..100);
                self.resource_manager.add_resource(x, y, resource_type.clone(), amount);
            });
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
            Self::valid_neighbors(x, y, self.width, self.height).for_each(|(nx, ny)| {
                if !visited[ny][nx] && !self.data[ny][nx] {
                    visited[ny][nx] = true;
                    queue.push_back((nx, ny));
                }
            });
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

    /// Ensures all walkable areas are connected by creating paths between isolated regions
    fn connect_isolated_regions(&mut self) {
        let mut visited = vec![vec![false; self.width]; self.height];

        // Identify all separate walkable regions.
        let mut regions: Vec<Vec<(usize, usize)>> = (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                if !self.data[y][x] && !visited[y][x] {
                    Some(self.collect_connected_walkable_cells(x, y, &mut visited))
                } else {
                    None
                }
            })
            .collect();

        // If there's only one region, the map is already fully connected
        if regions.len() <= 1 {
            return;
        }

        // Connect every other region to the first region
        let main_region = regions.remove(0);
        regions.into_iter().for_each(|region| {
            let (main_x, main_y) = main_region[0];
            let (other_x, other_y) = region[0];
            self.create_path(main_x, main_y, other_x, other_y);
        });
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

    pub fn get_resource(&self, x: usize, y: usize) -> Option<(crate::communication::channels::ResourceType, u32)> {
        self.resource_manager.get_resource(x, y)
            .map(|resource| {
                let channel_resource_type = match resource.resource_type {
                    crate::map::resources::ResourceType::Energy => 
                        crate::communication::channels::ResourceType::Energy,
                    crate::map::resources::ResourceType::Minerals => 
                        crate::communication::channels::ResourceType::Minerals,
                    crate::map::resources::ResourceType::SciencePoints => 
                        crate::communication::channels::ResourceType::SciencePoints,
                };
                (channel_resource_type, resource.amount)
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
    pub fn remove_resource(&mut self, x: usize, y: usize) -> Option<(crate::communication::channels::ResourceType, u32)> {
        if let Some(resource) = self.resource_manager.get_resource(x, y) {
            let resource_clone = resource.clone();
            let is_consumable = match resource_clone.resource_type {
                crate::map::resources::ResourceType::Energy | 
                crate::map::resources::ResourceType::Minerals => true,
                crate::map::resources::ResourceType::SciencePoints => false,
            };
            
            let channel_resource_type = match resource_clone.resource_type {
                crate::map::resources::ResourceType::Energy => 
                    crate::communication::channels::ResourceType::Energy,
                crate::map::resources::ResourceType::Minerals => 
                    crate::communication::channels::ResourceType::Minerals,
                crate::map::resources::ResourceType::SciencePoints => 
                    crate::communication::channels::ResourceType::SciencePoints,
            };
            
            if is_consumable {
                self.resource_manager.remove_resource(x, y);
            }
            
            Some((channel_resource_type, resource_clone.amount))
        } else {
            None
        }
    }

    pub fn get_all_resources(&self) -> &HashMap<(usize, usize), Resource> {
        self.resource_manager.get_all_resources()
    }

    /// Checks if the given coordinates contain a resource
    pub fn has_resource(&self, x: usize, y: usize) -> bool {
        self.resource_manager.has_resource(x, y)
    }

    /// Checks if the position contains an obstacle
    pub fn is_obstacle(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return true; // Out of bounds is considered an obstacle
        }
        self.data[y][x]
    }
}


// Formats the `Map` as a grid of characters (`#` for obstacles, `.` for walkable tiles)
impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resources = self.resource_manager.get_all_resources();
        
        for y in 0..self.height {
            for x in 0..self.width {
                let char = if self.data[y][x] {
                    '#'
                } else if let Some(resource) = resources.get(&(x, y)) {
                    match resource.resource_type {
                        ResourceType::Energy => 'E',
                        ResourceType::Minerals => 'M',
                        ResourceType::SciencePoints => 'S',
                    }
                } else {
                    '.'
                };
                write!(f, "{}", char)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
