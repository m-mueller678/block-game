use super::atomic_light::LightState;
use block::AtomicBlockId;
use num::Integer;
use world::BlockPos;
use std::cmp::max;
use std::ops::{Index, IndexMut};
use std::sync::atomic::AtomicBool;

pub const CHUNK_SIZE: usize = 32;

#[derive(Default)]
pub struct ChunkArray<T>([[[T; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]);

impl<T> Index<BlockPos> for ChunkArray<T> {
    type Output = T;
    fn index(&self, idx: BlockPos) -> &T {
        let cs = CHUNK_SIZE as i32;
        &self.0[idx[0].mod_floor(&cs) as usize][idx[1].mod_floor(&cs) as usize]
            [idx[2].mod_floor(&cs) as usize]
    }
}

impl<T> IndexMut<BlockPos> for ChunkArray<T> {
    fn index_mut(&mut self, idx: BlockPos) -> &mut T {
        let cs = CHUNK_SIZE as i32;
        &mut self.0[idx[0].mod_floor(&cs) as usize][idx[1].mod_floor(&cs) as usize][idx[2]
            .mod_floor(
                &cs,
            ) as
            usize]
    }
}

impl<T> Index<[usize; 3]> for ChunkArray<T> {
    type Output = T;
    fn index(&self, idx: [usize; 3]) -> &T {
        &self.0[idx[0]][idx[1]][idx[2]]
    }
}

impl<T> IndexMut<[usize; 3]> for ChunkArray<T> {
    fn index_mut(&mut self, idx: [usize; 3]) -> &mut T {
        &mut self.0[idx[0]][idx[1]][idx[2]]
    }
}

pub struct Chunk {
    pub data: ChunkArray<AtomicBlockId>,
    pub artificial_light: ChunkArray<LightState>,
    pub natural_light: ChunkArray<LightState>,
    pub is_in_update_queue: AtomicBool,
}

impl Chunk {
    pub fn effective_light(&self, pos: [usize; 3]) -> u8 {
        max(
            self.artificial_light[pos].level(),
            self.natural_light[pos].level(),
        )
    }
}
