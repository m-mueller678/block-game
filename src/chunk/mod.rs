mod direction;
mod block_draw;

use std::cell::Cell;
use block::BlockId;

pub use self::block_draw::{ChunkUniforms, init_chunk_shader, RenderChunk, block_graphics_supplier};
pub use self::direction::{Direction};

pub const CHUNK_SIZE: usize = 32;

type ChunkBlockData = [[[BlockId; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

pub struct Chunk {
    data: ChunkBlockData,
    changed: Cell<bool>
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            data: [[[BlockId::empty(); CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
            changed: Cell::new(true),
        }
    }
    pub fn set_block(&mut self, pos: &[usize; 3], block: BlockId) {
        self.changed.set(true);
        self.data[pos[0]][pos[1]][pos[2]] = block;
    }
}