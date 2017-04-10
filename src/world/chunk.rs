use super::atomic_light::LightState;
use block::{AtomicBlockId, BlockId};
use std::sync::atomic::AtomicBool;
use num::Integer;
use world::BlockPos;
use std::cmp::max;

pub const CHUNK_SIZE: usize = 32;

pub struct Chunk {
    pub data: [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    pub artificial_light: [LightState; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    pub natural_light: [LightState; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    pub update_render: AtomicBool,
}

pub struct ChunkReader<'a> {
    chunk: &'a Chunk,
}

pub fn chunk_index_global(p: &BlockPos) -> usize {
    let cs = CHUNK_SIZE as i32;
    p[0].mod_floor(&cs) as usize * CHUNK_SIZE * CHUNK_SIZE
        + p[1].mod_floor(&cs) as usize * CHUNK_SIZE
        + p[2].mod_floor(&cs) as usize
}

pub fn chunk_index(p: &[usize; 3]) -> usize {
    p[0] * CHUNK_SIZE * CHUNK_SIZE + p[1] * CHUNK_SIZE + p[2]
}

impl<'a> ChunkReader<'a> {
    pub fn new(chunk: &'a Chunk) -> Self {
        ChunkReader {
            chunk: chunk
        }
    }
    pub fn block(&self, pos: usize) -> BlockId {
        self.chunk.data[pos].load()
    }
    pub fn effective_light(&self, pos: usize) -> u8 {
        max(self.chunk.artificial_light[pos].level(), self.chunk.natural_light[pos].level())
    }
}