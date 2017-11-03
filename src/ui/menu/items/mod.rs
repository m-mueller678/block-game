use graphics::VirtualDisplay;
use ui::UiCore;
use item::Slot;
use module::GameData;

pub use self::item_count_render::ItemCountRender;
pub use self::inventory_ui::InventoryUi;

mod item_count_render;
mod inventory_ui;

#[derive(Clone)]
pub struct ItemSlotRender {
    count: Option<ItemCountRender>,
}

impl ItemSlotRender {
    pub fn new() -> Self {
        ItemSlotRender {
            count: None
        }
    }
    pub fn render<D: VirtualDisplay>(
        &mut self,
        slot: &Slot,
        gd: &GameData,
        ui_core: &UiCore,
        display: &mut D
    ) {
        let mut lock=slot.lock();
        if let Some(ref mut item) = lock.stack() {
            item.render(gd, ui_core, display);
            if item.display_stack_size() {
                let count = self.count.get_or_insert_with(|| ItemCountRender::new(ui_core));
                count.update(item.stack_size());
                count.render(display);
            } else {
                self.count = None
            }
        }
    }
}
