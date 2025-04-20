use crate::map::noise::Map;
use crate::robot::knowledge::{RobotKnowledge, TileInfo};
use crate::robot::movement::{self, Direction};

pub fn move_towards_target(
    current_x: usize,
    current_y: usize,
    target_x: usize,
    target_y: usize,
    knowledge: &RobotKnowledge,
    map: &Map,
) -> Direction {
    let dx = target_x as isize - current_x as isize;
    let dy = target_y as isize - current_y as isize;

    // Generate potential primary directions based on largest distance
    let primary_dirs = if dx.abs() > dy.abs() {
        if dx > 0 {
            vec![Direction::Right, Direction::Left]
        } else {
            vec![Direction::Left, Direction::Right]
        }
    } else {
        if dy > 0 {
            vec![Direction::Down, Direction::Up]
        } else {
            vec![Direction::Up, Direction::Down]
        }
    };

    // Generate potential secondary directions
    let secondary_dirs = if dx.abs() <= dy.abs() {
        if dx > 0 {
            vec![Direction::Right, Direction::Left]
        } else {
            vec![Direction::Left, Direction::Right]
        }
    } else {
        if dy > 0 {
            vec![Direction::Down, Direction::Up]
        } else {
            vec![Direction::Up, Direction::Down]
        }
    };

    // Check primary directions first for non-obstacle paths
    for &dir in &primary_dirs {
        let (nx, ny) = movement::next_position(current_x, current_y, &dir, map);
        if (nx, ny) != (current_x, current_y)
            && !matches!(knowledge.get_tile(nx, ny), TileInfo::Obstacle)
        {
            return dir;
        }
    }

    // Check secondary directions if primary are blocked by known obstacles
    for &dir in &secondary_dirs {
        let (nx, ny) = movement::next_position(current_x, current_y, &dir, map);
        if (nx, ny) != (current_x, current_y)
            && !matches!(knowledge.get_tile(nx, ny), TileInfo::Obstacle)
        {
            return dir;
        }
    }

    primary_dirs[0]
}
