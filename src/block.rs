use graphics::DrawType;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct BlockId(u32);

impl Default for BlockId {
    fn default() -> BlockId {
        BlockId::empty()
    }
}

#[derive(Default)]
pub struct AtomicBlockId(AtomicU32);

impl AtomicBlockId {
    pub fn new(id: BlockId) -> Self {
        AtomicBlockId(AtomicU32::new(id.0))
    }
    pub fn store(&self, id: BlockId) {
        self.0.store(id.0, Ordering::Relaxed);
    }
    pub fn load(&self) -> BlockId {
        BlockId(self.0.load(Ordering::Relaxed))
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

impl LightType {
    pub fn is_opaque(&self) -> bool {
        match *self {
            LightType::Opaque => true,
            LightType::Transparent | LightType::Source(_) => false,
        }
    }
}

pub struct Block {
    draw: DrawType,
    light: LightType,
    name: String,
}

impl Block {
    pub fn new(draw: DrawType, light: LightType, name: String) -> Self {
        Block {
            draw: draw,
            light: light,
            name: name,
        }
    }
}

pub struct BlockRegistry {
    blocks: Vec<Block>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        BlockRegistry {
            blocks: vec![Block { draw: DrawType::None, light: LightType::Transparent, name: "empty".into() }]
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
    pub fn is_opaque_draw(&self, block_id: BlockId) -> bool {
        if let DrawType::FullOpaqueBlock(_) = self.draw_type(block_id) {
            true
        } else {
            false
        }
    }
    pub fn by_name(&self, name: &str) -> Option<BlockId> {
        self.blocks.iter()
            .enumerate()
            .filter(|&(_, block)| block.name == name)
            .map(|(i, _)| BlockId(i as u32))
            .next()
    }
}