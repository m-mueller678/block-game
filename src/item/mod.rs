use ui::UiCore;
use std::any::Any;
use module::GameData;
use graphics::VirtualDisplay;

pub use self::block_item::BlockItem;
pub use self::storage::{SlotStorage, Slot};

mod block_item;
mod storage;

pub trait ItemStack where Self: Send {
    fn render(&self, &GameData, &UiCore, &mut VirtualDisplay);

    ///move items from from to self
    ///remaining items are returned
    ///inventory_stack_size_multiplier may be very large, consider using u32::saturating_mul
    fn stack_from(&mut self, gd:&GameData, from:Box<ItemStack>,inventory_stack_size_multiplier:u32) -> Option<Box<ItemStack>>;

    ///move at most max items from from to self
    ///remaining items are returned
    ///inventory_stack_size_multiplier may be very large, consider using u32::saturating_mul
    fn stack_some_from(&mut self, gd:&GameData, mut from:Box<ItemStack>, inventory_stack_size_multiplier:u32, max:u32) -> Option<Box<ItemStack>>{
        let count=from.count();
        if count==0{
            Some(from)
        }else if count<=max{
            self.stack_from(gd,from,inventory_stack_size_multiplier)
        }else{
            let new_stack=from.take(gd,max);
            if let Some(rest)=self.stack_from(gd,new_stack,inventory_stack_size_multiplier){
                let back_rest=from.stack_from(gd,rest,u32::max_value());
                assert!(back_rest.is_none());
            }
            Some(from)
        }
    }

    ///transfer item_count items to new stack
    ///implementation may assume 0 < item_count < self.count()
    fn take(&mut self,gd:&GameData,item_count:u32)->Box<ItemStack>;

    fn count(&self) -> u32;

    ///true if self.count() should be displayed in inventories in addition to self.render()
    fn display_stack_size(&self) -> bool { true }
    fn as_any(&self)->&Any;
    fn as_any_mut(&mut self)->&mut Any;
}