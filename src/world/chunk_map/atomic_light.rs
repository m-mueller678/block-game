use geometry::Direction;
use std::sync::atomic::{Ordering, AtomicU8};

#[derive(Default)]
pub struct LightState {
    level: AtomicU8,
    direction: AtomicU8,
}

const NO_DIRECTION: u8 = 6;

impl LightState {
    pub fn level(&self) -> u8 {
        self.level.load(Ordering::Relaxed)
    }
    pub fn direction(&self) -> Option<Direction> {
        let raw = self.direction.load(Ordering::Relaxed);
        if raw == NO_DIRECTION {
            None
        } else {
            Some(Direction::from_usize(raw as usize))
        }
    }
    pub fn set(&self, level: u8, direction: Option<Direction>) {
        self.level.store(level, Ordering::Relaxed);
        self.direction.store(
            direction.map(|d| d as u8).unwrap_or(NO_DIRECTION),
            Ordering::Relaxed,
        );
    }
}
