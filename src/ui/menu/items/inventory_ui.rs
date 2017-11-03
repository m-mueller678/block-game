use graphics::VirtualDisplay;
use ui::UiCore;
use item::{SlotStorage, Slot};
use geometry::Rectangle;
use module::GameData;
use super::super::Accessor;
use super::ItemStackRender;

pub struct InventoryUi<A: Accessor<SlotStorage>> {
    width: usize,
    game_data: GameData,
    item_renders: Vec<ItemStackRender>,
    storage: A,
}


impl<A: Accessor<SlotStorage>> InventoryUi<A> {
    pub fn new(width: usize, game_data: GameData, storage: A) -> Self {
        InventoryUi { width, game_data, storage, item_renders: Vec::new() }
    }

    pub fn render<D: VirtualDisplay>(&mut self, display: &mut D, ui_core: &UiCore) {
        if let Some(storage) = self.storage.get() {
            let storage_len = storage.len();
            self.item_renders.resize(storage_len, ItemStackRender::new());
            let item_size_x = 1. / self.width as f32;
            let item_size_y = 1. / Self::height(&*storage, self.width) as f32;
            for i in 0..storage_len {
                let pos_x = (i % self.width) as f32 * item_size_x;
                let pos_y = (i / self.width) as f32 * item_size_y;
                let rect_slot = Rectangle {
                    min_x: pos_x,
                    max_x: pos_x + item_size_x,
                    min_y: pos_y,
                    max_y: pos_y + item_size_y,
                };
                {
                    display.sub_display(rect_slot).fill_with_texture(self.game_data.core_textures().ui_item_slot, 1.);
                }
                if let &Some(ref item) = storage[i].stack() {
                    let rect_item = Rectangle {
                        min_x: rect_slot.min_x + item_size_x / 8.,
                        max_x: rect_slot.max_x - item_size_x / 8.,
                        min_y: rect_slot.min_y + item_size_y / 8.,
                        max_y: rect_slot.max_y - item_size_y / 8.,
                    };
                    let mut display = display.sub_display(rect_item);
                    self.item_renders[i].render(&**item, &self.game_data, ui_core, &mut display);
                }
            }
        }
    }

    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    pub fn size(&mut self) -> (f32, f32) {
        if let Some(storage) = self.storage.get() {
            (self.width as f32, Self::height(&*storage, self.width) as f32)
        } else {
            (self.width as f32, 0.)
        }
    }

    pub fn click(&mut self, x: f32, y: f32, holding: &mut Slot) {
        if let Some(mut storage) = self.storage.get_mut() {
            let slot = Self::slot_at(x, y, &*storage, self.width);
            if slot < storage.len() {
                if holding.is_none() {
                    holding.move_from(&self.game_data, &mut storage[slot]);
                } else {
                    storage[slot].move_from(&self.game_data, holding);
                }
            }
        }
    }

    fn height(storage: &SlotStorage, width: usize) -> usize {
        (storage.len() + width - 1) / width
    }

    fn slot_at(x: f32, y: f32, storage: &SlotStorage, width: usize) -> usize {
        (x * width as f32) as usize
            + width * (y * Self::height(storage, width) as f32) as usize
    }
}
