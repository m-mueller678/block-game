mod direction;
mod block_draw;

pub use self::block_draw::{ChunkUniforms, init_chunk_shader, RenderChunk, BlockGraphicsSupplier, BlockTextureId};
pub use self::direction::{Direction};

pub const CHUNK_SIZE: usize = 32;

type ChunkBlockData = [[[u32; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

pub struct Chunk {
    data: ChunkBlockData,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk { data: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE] }
    }
    pub fn set_block(&mut self, pos: &[usize; 3], block: u32) {
        self.data[pos[0]][pos[1]][pos[2]] = block;
    }
}