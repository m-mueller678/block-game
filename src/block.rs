use chunk::block_graphics_supplier::{DrawType, BlockGraphicsSupplier};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct BlockId(u32);

impl BlockId {
    pub fn empty() -> Self {
        BlockId(0)
    }
}

pub struct Block {
    draw: DrawType,
}

impl Block {
    pub fn new(draw: DrawType) -> Self {
        Block {
            draw: draw
        }
    }
}

pub struct BlockRegistry {
    blocks: Vec<Block>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        BlockRegistry {
            blocks: vec![Block { draw: DrawType::None }]
        }
    }
    pub fn add(&mut self, block: Block) -> BlockId {
        self.blocks.push(block);
        BlockId(self.blocks.len() as u32 - 1)
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