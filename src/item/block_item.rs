use block::BlockId;
use module::GameData;
use ui::UiCore;
use geometry::Rectangle;
use super::*;

pub struct BlockItem {
    block_id: BlockId,
}

impl BlockItem {
    pub fn new(block: BlockId, count: u32) -> Self {
        let _ =count;
        BlockItem {
            block_id: block,
        }
    }
}

impl ItemStack for BlockItem {
    fn render(&self, game_data: &GameData, _: &UiCore, render_buffer: &mut RenderBuffer2d, position: &Rectangle<f32>) {
        let transform_pos = |p: [f32; 2]| {
            [
                position.left + (position.right - position.left) * p[0],
                position.top + (position.bottom - position.top) * p[1],
            ]
        };
        let transform_quad = |q: [[f32; 2]; 4]| {
            [transform_pos(q[0]), transform_pos(q[1]), transform_pos(q[2]), transform_pos(q[3])]
        };
        use graphics::DrawType;
        use geometry::Direction;
        const H: f32 = 0.3;
        const W: f32 = 0.3;
        const D: f32 = 0.4243;//hypot(h,w)
        match game_data.blocks().draw_type(self.block_id) {
            DrawType::None => {}
            DrawType::FullOpaqueBlock(textures) => {
                render_buffer.push_quad(
                    transform_quad([[0., 2. * H], [W, H], [0., 0.], [-W, H]]),
                    [[0., 1.], [0., 0.], [1., 0.], [1., 1.]],
                    textures[Direction::PosY as usize],
                    0.4
                );
                render_buffer.push_quad(
                    transform_quad([[0., 2. * H], [-W, H], [-W, H + D], [0., 2. * H + D]]),
                    [[1., 1.], [0., 1.], [0., 0.], [1., 0.]],
                    textures[Direction::PosX as usize],
                    0.4*0.5
                );
                render_buffer.push_quad(
                    transform_quad([[W, H], [0., 2. * H], [0., 2. * H + D], [W, H + D]]),
                    [[1., 1.], [0., 1.], [0., 0.], [1., 0.]],
                    textures[Direction::PosX as usize],
                    0.4*0.25
                );
            }
        }
    }
    fn stack_from(&mut self, _: &GameData, _: Box<ItemStack>) -> Option<Box<ItemStack>> {
        unimplemented!()
    }
}