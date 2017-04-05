mod direction;
mod block_draw;

use num::Integer;
use block::BlockId;

pub use self::block_draw::{ChunkUniforms, init_chunk_shader, RenderChunk, block_graphics_supplier};
pub use self::direction::{Direction, ALL_DIRECTIONS};

pub const CHUNK_SIZE: usize = 32;

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