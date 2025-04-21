pub mod behavior {
    pub mod collection;
    pub mod exploration;
    pub mod scientific;
}

pub mod core {
    pub mod knowledge;
    pub mod movement;
    pub mod state;
}

pub mod utils {
    pub mod common;
    pub mod config;
}

// Re-export commonly used types if needed
pub use core::state::RobotState;
