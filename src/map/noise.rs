use noise::{NoiseFn, Perlin};
use std::collections::VecDeque;
use std::fmt;

pub struct Map {
    pub width: usize,
    pub height: usize,
    data: Vec<Vec<bool>>, // true = obstacle (#), false = walkable (.)
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

        let mut map = Self { width, height, data };
        map.connect_isolated_regions();
        map
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
    fn valid_neighbors(x: usize, y: usize, width: usize, height: usize) -> impl Iterator<Item = (usize, usize)> {
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
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Formats the `Map` as a grid of characters (`#` for obstacles, `.` for walkable tiles)
        self.data.iter().for_each(|row| {
            row.iter().for_each(|&cell| {
                let _ = write!(f, "{}", if cell { '#' } else { '.' });
            });
            let _ = writeln!(f);
        });
        Ok(())
    }
}
