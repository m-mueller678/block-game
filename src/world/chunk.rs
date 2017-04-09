use super::atomic_light::LightState;
use block::{AtomicBlockId, BlockId, BlockRegistry};
use std::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};
use num::Integer;
use world::BlockPos;

pub const CHUNK_SIZE: usize = 32;

pub struct Chunk {
    pub vertical_clear: [AtomicBool; CHUNK_SIZE * CHUNK_SIZE],
    pub data: [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    pub light: [LightState; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    pub update_render: AtomicBool,
}


pub fn update_vertical_clear(vertical_clear: &[AtomicBool; CHUNK_SIZE * CHUNK_SIZE],
                             data: &[AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
                             pos: [usize; 2],
                             blocks: &BlockRegistry)
                             -> bool {
    let opaque = (0..CHUNK_SIZE)
        .any(|y| blocks.light_type(data[chunk_index(&[pos[0], y, pos[1]])].load()).is_opaque());
    vertical_clear[chunk_xz_index(pos[0], pos[1])].store(!opaque, Ordering::Relaxed);
    !opaque
}

pub fn init_vertical_clear() -> [AtomicBool; CHUNK_SIZE * CHUNK_SIZE] {
    use std::mem::uninitialized;
    use std::ptr::write;
    unsafe {
        let mut ret: [AtomicBool; CHUNK_SIZE * CHUNK_SIZE] = uninitialized();
        for l in ret.iter_mut() {
            write(l, ATOMIC_BOOL_INIT)
        }
        ret
    }
}

pub struct ChunkReader<'a> {
    chunk: &'a Chunk,
}

pub fn chunk_xz_index(x: usize, z: usize) -> usize {
    x * CHUNK_SIZE + z
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
    pub fn light(&self, pos: usize) -> u8 {
        self.chunk.light[pos].level()
    }
}