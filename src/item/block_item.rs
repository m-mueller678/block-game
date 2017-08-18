use block::BlockId;
use module::GameData;
use ui::UiCore;
use super::*;

const MAX_STACK_SIZE: usize = 99;

pub struct BlockItem {
    block_id: BlockId,
    count: usize
}

impl BlockItem {
    pub fn new(block: BlockId, count: usize) -> Self {
        assert!(count <= MAX_STACK_SIZE);
        assert!(count > 0);
        BlockItem {
            block_id: block,
            count,
        }
    }
}

impl ItemStack for BlockItem {
    fn render(&self, game_data: &GameData, _: &UiCore, render_buffer: &mut VirtualDisplay) {
        use graphics::DrawType;
        use geometry::Direction;
        const H: f32 = 0.23;
        const W: f32 = 0.4;
        const D: f32 = 0.4614;//hypot(h,w)
        match game_data.blocks().draw_type(self.block_id) {
            DrawType::None => {}
            DrawType::FullOpaqueBlock(textures) => {
                render_buffer.textured_quad(
                    [[0.5, 2. * H], [0.5 + W, H], [0.5, 0.], [0.5 - W, H]],
                    [[0., 1.], [0., 0.], [1., 0.], [1., 1.]],
                    textures[Direction::PosY as usize],
                    0.4
                );
                render_buffer.textured_quad(
                    [[0.5, 2. * H], [0.5 - W, H], [0.5 - W, H + D], [0.5, 2. * H + D]],
                    [[1., 1.], [0., 1.], [0., 0.], [1., 0.]],
                    textures[Direction::PosX as usize],
                    0.4 * 0.5
                );
                render_buffer.textured_quad(
                    [[0.5 + W, H], [0.5, 2. * H], [0.5, 2. * H + D], [0.5 + W, H + D]],
                    [[1., 1.], [0., 1.], [0., 0.], [1., 0.]],
                    textures[Direction::PosX as usize],
                    0.4 * 0.25
                );
            }
        }
    }
    fn stack_from(&mut self, _: &GameData, _: Box<ItemStack>) -> Option<Box<ItemStack>> {
        unimplemented!()
    }
    fn stack_size(&self) -> usize {
        self.count
    }
}