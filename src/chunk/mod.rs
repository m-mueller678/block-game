use num::Integer;
use block::BlockId;
use geometry::Direction;
use world::CHUNK_SIZE;

pub struct Chunk {
    pub data: [(BlockId); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    pub light: [(u8, Direction); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE]
}

impl Chunk {
    pub fn index(pos: &[i32; 3]) -> usize {
        pos[0].mod_floor(&(CHUNK_SIZE as i32)) as usize * CHUNK_SIZE * CHUNK_SIZE
            + pos[1].mod_floor(&(CHUNK_SIZE as i32)) as usize * CHUNK_SIZE
            + pos[2].mod_floor(&(CHUNK_SIZE as i32)) as usize
    }
    pub fn u_index(pos: &[usize; 3]) -> usize {
        pos[0].mod_floor(&(CHUNK_SIZE)) * CHUNK_SIZE * CHUNK_SIZE
            + pos[1].mod_floor(&(CHUNK_SIZE)) * CHUNK_SIZE
            + pos[2].mod_floor(&(CHUNK_SIZE))
    }
}