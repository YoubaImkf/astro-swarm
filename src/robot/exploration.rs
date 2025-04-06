use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::communication::channels::RobotEvent;

pub struct ExplorationRobot {
    pub id: u32,
    pub x: usize,
    pub y: usize,
}

impl ExplorationRobot {
    pub fn new(id: u32, start_x: usize, start_y: usize) -> Self {
        Self { id, x: start_x, y: start_y }
    }

    /// Starts the exploration loop. The robot will periodically:
    /// - "Move" (simulated by a simple coordinate update)
    /// - Send its current position as exploration data.
    pub fn start(mut self, sender: Sender<RobotEvent>) {
        thread::spawn(move || {
            loop {
                // Simulate exploration behavior.
                // (A real implementation would include proper movement logic and
                // obstacle avoidance.)
                self.x = (self.x + 1) % 80;
                self.y = (self.y + 1) % 24;

                // Send exploration data
                let event = RobotEvent::ExplorationData {
                    id: self.id,
                    x: self.x,
                    y: self.y,
                    is_obstacle: false,
                };
                if sender.send(event).is_err() {
                    break;
                }

                thread::sleep(Duration::from_secs(1));
            }
        });
    }
}