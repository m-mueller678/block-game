use chunk::block_graphics_supplier::{DrawType, BlockGraphicsSupplier};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct BlockId(u32);

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
}

impl BlockGraphicsSupplier for BlockRegistry {
    fn get_draw_type(&self, block_id: BlockId) -> DrawType {
        self.blocks[block_id.0 as usize].draw.clone()
    }

    fn is_opaque(&self, block_id: BlockId) -> bool {
        if let DrawType::FullOpaqueBlock(_) = self.get_draw_type(block_id) {
            true
        } else {
            false
        }
    }
}