use crate::map::noise::Map;
use crate::robot::core::knowledge::{RobotKnowledge, TileInfo};
use rand::seq::IndexedRandom;
use rand::{rng, Rng};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn all() -> [Self; 4] {
        [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
    }

    pub fn random() -> Self {
        let mut rng = rng();
        match rng.random_range(0..4) {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
        }
    }
}

pub fn next_position(x: usize, y: usize, dir: &Direction, map: &Map) -> (usize, usize) {
    match dir {
        Direction::Up if y > 0 => (x, y.saturating_sub(1)),
        Direction::Down if y < map.height.saturating_sub(1) => (x, y + 1),
        Direction::Left if x > 0 => (x.saturating_sub(1), y),
        Direction::Right if x < map.width.saturating_sub(1) => (x + 1, y),
        _ => (x, y),
    }
}

pub fn is_valid_move(x: usize, y: usize, map: &Map) -> bool {
    x < map.width && y < map.height && !map.is_obstacle(x, y)
}

pub fn smart_direction(
    x: usize,
    y: usize,
    knowledge: &RobotKnowledge,
    visited_in_cycle: &HashSet<(usize, usize)>,
    map: &Map,
) -> Option<Direction> {
    let mut rng = rng();
    let mut resource_candidates = Vec::new();
    let mut walkable_candidates = Vec::new();
    let mut fallback_candidates = Vec::new();

    for dir in Direction::all().iter() {
        let (nx, ny) = next_position(x, y, dir, map);

        if (nx, ny) == (x, y) {
            continue;
        }

        let tile = knowledge.get_tile(nx, ny);
        match tile {
            TileInfo::Obstacle | TileInfo::Unknown => continue,
            TileInfo::Resource(_, amount) if *amount > 0 => {
                if !visited_in_cycle.contains(&(nx, ny)) {
                    resource_candidates.push(*dir);
                } else {
                    fallback_candidates.push(*dir);
                }
            }
            TileInfo::Resource(_, 0) | TileInfo::Walkable | TileInfo::Station => {
                if !visited_in_cycle.contains(&(nx, ny)) {
                    walkable_candidates.push(*dir);
                } else {
                    fallback_candidates.push(*dir);
                }
            }

            _ => continue,
        }
    }

    // Prioritize unvisited resources
    if !resource_candidates.is_empty() {
        return resource_candidates.choose(&mut rng).copied();
    }
    // Then unvisited walkable tiles
    if !walkable_candidates.is_empty() {
        return walkable_candidates.choose(&mut rng).copied();
    }
    // Finally, already visited
    if !fallback_candidates.is_empty() {
        return fallback_candidates.choose(&mut rng).copied();
    }

    None
}

/// UI/Animation
#[derive(Debug, Clone, Copy)]
pub struct SmoothPos {
    pub x: f32,
    pub y: f32,
}

impl SmoothPos {
    pub fn new(x: usize, y: usize) -> Self {
        Self {
            x: x as f32,
            y: y as f32,
        }
    }
    pub fn move_towards(&mut self, tx: usize, ty: usize, speed: f32) {
        let dx = (tx as f32) - self.x;
        let dy = (ty as f32) - self.y;
        let dist = dx * dx + dy * dy;

        if dist > 0.0001 {
            let dist = dist.sqrt();
            let move_dist = speed.min(dist);

            self.x += (dx / dist) * move_dist;
            self.y += (dy / dist) * move_dist;
        } else {
            self.x = tx as f32;
            self.y = ty as f32;
        }
    }
}
