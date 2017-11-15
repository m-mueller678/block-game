use super::*;
use world::BlockPos;

pub struct LightDecrease {
    dec: [Vec<(BlockPos, Option<LightDirection>)>; MAX_LIGHT as usize + 1],
    max: usize,
}

impl LightDecrease {
    pub fn new() -> Self {
        LightDecrease {
            dec: Default::default(),
            max: MAX_LIGHT as usize,
        }
    }

    pub fn pop(&mut self) -> Option<(BlockPos, Option<LightDirection>)> {
        self.move_max().map(|max| self.dec[max].pop().unwrap())
    }

    fn move_max(&mut self) -> Option<usize> {
        while self.dec[self.max].is_empty() {
            if self.max == 0 {
                return None;
            } else {
                self.max -= 1;
            }
        }
        Some(self.max)
    }

    /// the light level must not be greater than the one returned by the last call to pop
    /// unless reset has been called since then
    /// if pop returned None reset must be called before push
    pub fn push(&mut self, pos: BlockPos, level: u8, direction_filter: Option<LightDirection>) {
        let index = level as usize;
        debug_assert!(index <= self.max);
        self.dec[index].push((pos, direction_filter))
    }

    pub fn reset(&mut self) {
        self.max = MAX_LIGHT as usize;
    }
}
