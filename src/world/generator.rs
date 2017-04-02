use md5;

use chunk::{Chunk, CHUNK_SIZE};
use block::BlockId;

pub struct Generator {
    ground: BlockId,
}

impl Generator {
    pub fn new(ground: BlockId) -> Self {
        Generator {
            ground: ground,
        }
    }

    pub fn gen_chunk(&self, pos: &[i32; 3]) -> Chunk {
        let mut ret = Chunk::new();
        let bottom = pos[1] * CHUNK_SIZE as i32;
        let surface = Self::surface_to_abs_height(self.base_height(pos[0], pos[2]));
        for y in 0..CHUNK_SIZE {
            if !(bottom + (y as i32) < surface) {
                break;
            }
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    ret.set_block(&[x, y, z], self.ground);
                }
            }
        }
        ret
    }

    fn raw_height(&self, x: i32, z: i32) -> u8 {
        let mut context = md5::Context::new();
        let (pos0, pos1) = unsafe {
            use std::mem::transmute;
            (transmute::<i32, [u8; 4]>(x.to_le()),
             transmute::<i32, [u8; 4]>(z.to_le()), )
        };
        context.consume(&pos0);
        context.consume(&pos1);
        context.compute()[0]
    }

    fn base_height(&self, x: i32, z: i32) -> u8 {
        let self_height = self.raw_height(x, z) as u32;
        let mut total_surrounding_height = 0;
        total_surrounding_height += self.raw_height(x - 1, z - 1) as u32;
        total_surrounding_height += self.raw_height(x - 1, z) as u32;
        total_surrounding_height += self.raw_height(x - 1, z + 1) as u32;
        total_surrounding_height += self.raw_height(x, z - 1) as u32;
        total_surrounding_height += self.raw_height(x, z + 1) as u32;
        total_surrounding_height += self.raw_height(x + 1, z - 1) as u32;
        total_surrounding_height += self.raw_height(x + 1, z) as u32;
        total_surrounding_height += self.raw_height(x + 1, z + 1) as u32;
        ((self_height + total_surrounding_height / 4) / 3) as u8
    }

    fn surface_to_abs_height(h: u8) -> i32 {
        h as i32 * CHUNK_SIZE as i32 / 256
    }
}