use geometry::Direction;
use std::sync::atomic::{Ordering, AtomicU8};
use super::lighting::LightDirection;

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
    pub fn direction(&self) -> LightDirection {
        let raw = self.direction.load(Ordering::Relaxed);
        if raw == NO_DIRECTION {
            LightDirection::SelfLit
        } else {
            LightDirection::Directed(Direction::from_usize(raw as usize))
        }
    }
    pub fn set(&self, level: u8, direction: LightDirection) {
        self.level.store(level, Ordering::Relaxed);
        self.direction.store(
            match direction {
                LightDirection::SelfLit => NO_DIRECTION,
                LightDirection::Directed(d) => d as u8,
            },
            Ordering::Relaxed,
        );
    }
}
