use block::BlockId;
use module::GameData;
use ui::UiCore;
use super::*;

const MAX_STACK_SIZE: u32 = 100;

pub struct BlockItem {
    block_id: BlockId,
    count: u32,
}

impl BlockItem {
    pub fn new(block: BlockId, count: u32) -> Self {
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
        const D: f32 = 0.4614; //hypot(h,w)
        match game_data.blocks().draw_type(self.block_id) {
            DrawType::None => {}
            DrawType::FullOpaqueBlock(textures) => {
                render_buffer.textured_quad(
                    [[0.5, 2. * H], [0.5 + W, H], [0.5, 0.], [0.5 - W, H]],
                    [[0., 1.], [0., 0.], [1., 0.], [1., 1.]],
                    textures[Direction::PosY as usize],
                    0.4,
                );
                render_buffer.textured_quad(
                    [
                        [0.5, 2. * H],
                        [0.5 - W, H],
                        [0.5 - W, H + D],
                        [0.5, 2. * H + D],
                    ],
                    [[1., 1.], [0., 1.], [0., 0.], [1., 0.]],
                    textures[Direction::PosX as usize],
                    0.4 * 0.5,
                );
                render_buffer.textured_quad(
                    [
                        [0.5 + W, H],
                        [0.5, 2. * H],
                        [0.5, 2. * H + D],
                        [0.5 + W, H + D],
                    ],
                    [[1., 1.], [0., 1.], [0., 0.], [1., 0.]],
                    textures[Direction::PosX as usize],
                    0.4 * 0.25,
                );
            }
        }
    }
    fn stack_from(
        &mut self,
        _: &GameData,
        mut from: Box<ItemStack>,
        inventory_stack_size_multiplier: u32,
    ) -> Option<Box<ItemStack>> {
        let max_stack_size = MAX_STACK_SIZE
            .saturating_mul(inventory_stack_size_multiplier)
            .min(u32::max_value() / 2);
        if self.count >= max_stack_size {
            return Some(from);
        }
        if let Some(from) = from.as_any_mut().downcast_mut::<BlockItem>() {
            if from.block_id == self.block_id {
                let sum = from.count + self.count;
                if sum > MAX_STACK_SIZE {
                    self.count = MAX_STACK_SIZE;
                    from.count = sum - MAX_STACK_SIZE;
                } else {
                    self.count = sum;
                    return None;
                }
            }
        }
        Some(from)
    }
    fn take(&mut self, _: &GameData, item_count: u32) -> Box<ItemStack> {
        assert!(item_count < self.count);
        self.count -= item_count;
        Box::new(BlockItem {
            count: item_count,
            ..*self
        })
    }
    fn count(&self) -> u32 {
        self.count
    }
    fn as_any(&self) -> &Any {
        self as &Any
    }
    fn as_any_mut(&mut self) -> &mut Any {
        self as &mut Any
    }
}
