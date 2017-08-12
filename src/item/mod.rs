use ui::UiCore;
use std::any::Any;
use module::GameData;
use graphics::VirtualDisplay;

pub use self::block_item::BlockItem;

mod block_item;
mod storage;

pub trait ItemStack where Self:Any{
    fn render(&self,&GameData,&UiCore,&mut VirtualDisplay);
    fn stack_from(&mut self,&GameData,Box<ItemStack>)->Option<Box<ItemStack>>;
}