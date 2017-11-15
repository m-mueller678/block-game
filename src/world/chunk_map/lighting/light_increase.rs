use super::*;
use world::BlockPos;

pub struct LightIncrease {
    inc: [Vec<(BlockPos, LightDirection)>; MAX_LIGHT as usize + 1],
    max: usize,
}

impl LightIncrease {
    pub fn new() -> Self {
        LightIncrease {
            inc: Default::default(),
            max: MAX_LIGHT as usize,
        }
    }

    pub fn pop(&mut self) -> Option<(BlockPos, Light)> {
        while self.inc[self.max].is_empty() {
            if self.max == 0 {
                return None;
            } else {
                self.max -= 1;
            }
        }
        let ret = self.inc[self.max].pop().unwrap();
        Some((ret.0, (self.max as u8, ret.1)))
    }

    /// the light level must not be greater than the one returned by the last call to pop
    /// unless reset has been called since then
    /// if pop returned None reset must be called before push
    pub fn push(&mut self, pos: BlockPos, light: Light) {
        let index = light.0 as usize;
        debug_assert!(index <= self.max);
        self.inc[index].push((pos, light.1))
    }

    pub fn reset(&mut self) {
        self.max = MAX_LIGHT as usize;
    }
}
