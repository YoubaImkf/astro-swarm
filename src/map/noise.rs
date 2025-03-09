use noise::{NoiseFn, Perlin};
use std::collections::VecDeque;
use std::fmt;

pub struct Map {
    pub width: usize,
    pub height: usize,
    data: Vec<Vec<bool>>, // true = obstacle (#), false = walkable (.)
}

impl Map {
    pub fn new(width: usize, height: usize, seed: u32) -> Self {
        let perlin = Perlin::new(seed);
        let mut data = vec![vec![false; width]; height];

        for y in 0..height {
            for x in 0..width {
                let noise_value = perlin.get([x as f64 / 10.0, y as f64 / 10.0]);
                data[y][x] = noise_value > 0.0;
            }
        }

        let mut map = Self {
            width,
            height,
            data,
        };
        map.connect_isolated_regions();
        map
    }

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

    fn valid_neighbors(x: usize, y: usize, width: usize, height: usize) -> Vec<(usize, usize)> {
        let mut neighbors = Vec::new();
        if x > 0 {
            neighbors.push((x - 1, y));
        }
        if x < width - 1 {
            neighbors.push((x + 1, y));
        }
        if y > 0 {
            neighbors.push((x, y - 1));
        }
        if y < height - 1 {
            neighbors.push((x, y + 1));
        }
        neighbors
    }

    /// Ensures that all walkable areas are connected by carving paths between isolated regions
    fn connect_isolated_regions(&mut self) {
        let mut visited = vec![vec![false; self.width]; self.height];
        let mut regions = Vec::new();

        // Collect all distinct walkable regions
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.data[y][x] && !visited[y][x] {
                    let region = self.collect_connected_walkable_cells(x, y, &mut visited);
                    regions.push(region);
                }
            }
        }

        // If there is only one region, the map is already fully connected
        if regions.len() <= 1 {
            return;
        }

        // Connect every additional region to the first region
        let main_region = &regions[0];
        for region in regions.iter().skip(1) {
            let (main_x, main_y) = main_region[0];
            let (other_x, other_y) = region[0];
            self.create_path(main_x, main_y, other_x, other_y);
        }
    }

    fn create_path(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let (mut x, mut y) = (x1 as isize, y1 as isize);
        let (target_x, target_y) = (x2 as isize, y2 as isize);

        // Carve horizontal path
        while x != target_x {
            self.data[y as usize][x as usize] = false;
            x += if x < target_x { 1 } else { -1 };
        }
        // Carve vertical path
        while y != target_y {
            self.data[y as usize][x as usize] = false;
            y += if y < target_y { 1 } else { -1 };
        }
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.data {
            for &cell in row {
                let symbol = if cell { '#' } else { '.' };
                write!(f, "{}", symbol)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
