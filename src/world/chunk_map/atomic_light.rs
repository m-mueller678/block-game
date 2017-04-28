use geometry::Direction;
use std::sync::atomic::{Ordering, AtomicU8, ATOMIC_U8_INIT};
use world::CHUNK_SIZE;

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
        self.direction.store(direction.map(|d| d as u8).unwrap_or(NO_DIRECTION), Ordering::Relaxed);
    }
    pub fn init_dark_chunk() -> [LightState; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] {
        use std::mem::uninitialized;
        use std::ptr::write;
        unsafe {
            let mut array: [LightState; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] = uninitialized();
            for light in &mut array[..] {
                write(light, LightState { level: ATOMIC_U8_INIT, direction: ATOMIC_U8_INIT });
            }
            array
        }
    }
}