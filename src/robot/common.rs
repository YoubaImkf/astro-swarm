use crate::map::noise::Map;
use crate::robot::knowledge::{RobotKnowledge, TileInfo};
use crate::robot::movement::{is_valid_move, next_position, Direction};
use log::debug;

pub fn move_towards_target(
    current_x: usize,
    current_y: usize,
    target_x: usize,
    target_y: usize,
    knowledge: &RobotKnowledge,
    map: &Map,
) -> Direction {
    debug!(
        "Moving from ({},{}) towards ({},{})",
        current_x, current_y, target_x, target_y
    );

    let try_horizontal = if target_x > current_x {
        Some(Direction::Right)
    } else if target_x < current_x {
        Some(Direction::Left)
    } else {
        None
    };

    let try_vertical = if target_y > current_y {
        Some(Direction::Down)
    } else if target_y < current_y {
        Some(Direction::Up)
    } else {
        None
    };

    let directions_to_try = vec![
        try_horizontal,
        try_vertical,
        Some(Direction::Up),
        Some(Direction::Down),
        Some(Direction::Left),
        Some(Direction::Right),
    ];

    for dir_opt in directions_to_try {
        if let Some(dir) = dir_opt {
            let (nx, ny) = next_position(current_x, current_y, &dir, map);
            if (nx, ny) != (current_x, current_y) && // Ensure we actually move
               is_valid_move(nx, ny, map) &&
               !matches!(knowledge.get_tile(nx, ny), TileInfo::Obstacle)
            {
                debug!("Selected direction: {:?} -> new pos: ({},{})", dir, nx, ny);
                return dir;
            }
        }
    }

    for _ in 0..8 {
        let random_dir = Direction::random();
        let (nx, ny) = next_position(current_x, current_y, &random_dir, map);
        if (nx, ny) != (current_x, current_y)
            && is_valid_move(nx, ny, map)
            && !matches!(knowledge.get_tile(nx, ny), TileInfo::Obstacle)
        {
            debug!("Using random direction: {:?}", random_dir);
            return random_dir;
        }
    }

    debug!("No valid direction found, returning random");
    Direction::random()
}
