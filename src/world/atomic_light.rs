use geometry::Direction;
use std::sync::atomic::{Ordering, AtomicU8, ATOMIC_U8_INIT};
use world::CHUNK_SIZE;

pub struct LightState {
    level: AtomicU8,
    direction: AtomicU8,
}

impl LightState {
    pub fn level(&self) -> u8 {
        self.level.load(Ordering::Relaxed)
    }
    pub fn direction(&self) -> Direction {
        Direction::from_usize(self.direction.load(Ordering::Relaxed) as usize)
    }
    pub fn set(&self, level: u8, direction: Direction) {
        self.level.store(level, Ordering::Relaxed);
        self.direction.store(direction as u8, Ordering::Relaxed);
    }
    pub fn set_level(&self, l: u8) {
        self.level.store(l, Ordering::Relaxed);
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