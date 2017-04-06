use graphics::DrawType;
use std::sync::atomic::{AtomicU32, Ordering};
use world::CHUNK_SIZE;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct BlockId(u32);

pub struct AtomicBlockId(AtomicU32);

impl AtomicBlockId {
    pub fn store(&self, id: BlockId) {
        self.0.store(id.0, Ordering::Relaxed);
    }
    pub fn load(&self) -> BlockId {
        BlockId(self.0.load(Ordering::Relaxed))
    }
    pub fn init_chunk(data: &[BlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE])
                      -> [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] {
        use std::mem::uninitialized;
        use std::ptr::write;
        unsafe {
            let mut array: [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] = uninitialized();
            for i in 0..(array.len()) {
                write(&mut array[i], AtomicBlockId(AtomicU32::new(data[i].0)));
            }
            array
        }
    }
}

impl BlockId {
    pub fn empty() -> Self {
        BlockId(0)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum LightType {
    Transparent,
    Opaque,
    Source(u8),
}

pub struct Block {
    draw: DrawType,
    light: LightType,
}

impl Block {
    pub fn new(draw: DrawType, light: LightType) -> Self {
        Block {
            draw: draw,
            light: light,
        }
    }
}

pub struct BlockRegistry {
    blocks: Vec<Block>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        BlockRegistry {
            blocks: vec![Block { draw: DrawType::None, light: LightType::Transparent }]
        }
    }
    pub fn add(&mut self, block: Block) -> BlockId {
        self.blocks.push(block);
        BlockId(self.blocks.len() as u32 - 1)
    }
    pub fn light_type(&self, block_id: BlockId) -> &LightType {
        &self.blocks[block_id.0 as usize].light
    }
    pub fn draw_type(&self, block_id: BlockId) -> DrawType {
        self.blocks[block_id.0 as usize].draw.clone()
    }
    pub fn is_opaque(&self, block_id: BlockId) -> bool {
        if let DrawType::FullOpaqueBlock(_) = self.draw_type(block_id) {
            true
        } else {
            false
        }
    }
}