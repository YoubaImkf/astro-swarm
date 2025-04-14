use rand::Rng;
use crate::map::noise::Map;

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        match rng.random_range(0..4) {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
        }
    }
}

pub fn next_position(x: usize, y: usize, direction: &Direction, map: &Map) -> (usize, usize) {
    match direction {
        Direction::Up => (x, y.saturating_sub(1)),
        Direction::Down => (x, (y + 1).min(map.height - 1)),
        Direction::Left => (x.saturating_sub(1), y),
        Direction::Right => ((x + 1).min(map.width - 1), y),
    }
}

pub fn is_valid_move(x: usize, y: usize, map: &Map) -> bool {
    x < map.width && y < map.height && !map.is_obstacle(x, y)
}