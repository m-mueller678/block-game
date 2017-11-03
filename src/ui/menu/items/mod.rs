use graphics::VirtualDisplay;
use ui::UiCore;
use item::ItemStack;
use module::GameData;

pub use self::item_count_render::ItemCountRender;
pub use self::inventory_ui::InventoryUi;

mod item_count_render;
mod inventory_ui;

#[derive(Clone)]
pub struct ItemStackRender {
    count: Option<ItemCountRender>,
}

impl ItemStackRender {
    pub fn new() -> Self {
        ItemStackRender {
            count: None
        }
    }
    pub fn render<D: VirtualDisplay>(
        &mut self,
        item: &ItemStack,
        gd: &GameData,
        ui_core: &UiCore,
        display: &mut D
    ) {
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
