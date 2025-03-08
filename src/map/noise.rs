use noise::{NoiseFn, Perlin};
use std::fmt;

pub struct Map {
    pub width: usize,
    pub height: usize,
    data: Vec<Vec<bool>>,
}

impl Map {
    pub fn new(width: usize, height: usize, seed: u32) -> Self {
        let perlin = Perlin::new(seed);

        let mut data = vec![vec![false; width]; height];

        for y in 0..height {
            for x in 0..width {
                let value = perlin.get([x as f64 / 10.0, y as f64 / 10.0]);
                data[y][x] = value > 0.0;
            }
        }

        Map { width, height, data }
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