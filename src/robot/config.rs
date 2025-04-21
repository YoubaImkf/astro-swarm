use std::time::Duration;

/// Minimum sleep duration during the return-to-station phase (milliseconds)
pub const RETURN_SLEEP_MIN_MS: u64 = 150;
/// Maximum sleep duration during the return-to-station phase (milliseconds)
pub const RETURN_SLEEP_MAX_MS: u64 = 400;
/// Timeout duration for waiting for a MergeComplete message (seconds)
pub const MERGE_TIMEOUT: Duration = Duration::from_secs(3);
/// Default sleep duration when in the AtStation state (milliseconds)
pub const AT_STATION_SLEEP_MS: u64 = 100;
/// Default sleep duration when encountering an unhandled state (seconds)
pub const UNHANDLED_STATE_SLEEP: Duration = Duration::from_secs(1);

/// Max energy of each robots
pub const COLLECTION_ROBOT_MAX_ENERGY: u32 = 500;
pub const EXPLORATION_ROBOT_MAX_ENERGY: u32 = 800;
pub const SCIENTIFIC_ROBOT_MAX_ENERGY: u32 = 500;

#[derive(Debug, Clone)]
pub struct RobotTypeConfig {
    pub low_energy_threshold: u32,
    pub primary_action_sleep_min_ms: u64,
    pub primary_action_sleep_max_ms: u64,
    pub movement_energy_cost: u32,
    pub action_energy_cost: Option<u32>,
}

pub const EXPLORATION_CONFIG: RobotTypeConfig = RobotTypeConfig {
    low_energy_threshold: 20,
    primary_action_sleep_min_ms: 300,
    primary_action_sleep_max_ms: 600,
    movement_energy_cost: 1,
    action_energy_cost: None,
};

pub const COLLECTION_CONFIG: RobotTypeConfig = RobotTypeConfig {
    low_energy_threshold: 25,
    primary_action_sleep_min_ms: 400,
    primary_action_sleep_max_ms: 900,
    movement_energy_cost: 2,
    action_energy_cost: Some(3),
};

pub const SCIENTIFIC_CONFIG: RobotTypeConfig = RobotTypeConfig {
    low_energy_threshold: 30,
    primary_action_sleep_min_ms: 800,
    primary_action_sleep_max_ms: 1500,
    movement_energy_cost: 1,
    action_energy_cost: Some(5),
};

pub fn random_sleep_duration(min_ms: u64, max_ms: u64) -> Duration {
    use rand::{rng, Rng};
    if min_ms >= max_ms {
        Duration::from_millis(min_ms)
    } else {
        Duration::from_millis(rng().random_range(min_ms..=max_ms))
    }
}
