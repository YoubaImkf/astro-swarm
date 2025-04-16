use rand::Rng;
use std::collections::HashSet;
use crate::map::noise::Map;
use crate::robot::knowledge::{TileInfo, RobotKnowledge};

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn all() -> [Self; 4] {
        [Direction::Up, Direction::Down, Direction::Left, Direction::Right]
    }
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

/// Intelligent move selection:
/// - Prefer unvisited, walkable/resource neighbors.
/// - Avoid revisiting unless necessary.
/// - Prefer resource tiles when found.
/// - As fallback, allow revisiting already visited walkable/resource tiles.
/// - Avoid obstacles and unknowns.
pub fn smart_direction(
    x: usize,
    y: usize,
    knowledge: &RobotKnowledge,
    visited: &HashSet<(usize, usize)>,
    map: &Map,
) -> Option<Direction> {
    let mut rng = rand::rng();
    let mut candidates = Vec::new();
    let mut fallback = Vec::new();

    for dir in Direction::all().iter() {
        let (nx, ny) = next_position(x, y, dir, map);
        if (nx, ny) == (x, y) { continue; }
        let tile = knowledge.get_tile(nx, ny);
        match tile {
            TileInfo::Obstacle | TileInfo::Unknown => continue,
            TileInfo::Resource(_, _) => {
                if !visited.contains(&(nx, ny)) {
                    // Prefer new resources first
                    return Some(*dir);
                } else {
                    fallback.push(*dir);
                }
            }
            TileInfo::Walkable | TileInfo::Station => {
                if !visited.contains(&(nx, ny)) {
                    candidates.push(*dir);
                } else {
                    fallback.push(*dir);
                }
            }
        }
    }

    if !candidates.is_empty() {
        // Prefer random among unvisited options for natural exploration
        let idx = rng.random_range(0..candidates.len());
        return Some(candidates[idx]);
    }
    if !fallback.is_empty() {
        // All neighbors visited; pick one to avoid deadlock
        let idx = rng.random_range(0..fallback.len());
        return Some(fallback[idx]);
    }
    None
}