use ui::UiCore;
use std::any::Any;
use geometry::Rectangle;
use module::GameData;
use graphics::RenderBuffer2d;

pub use self::block_item::BlockItem;

mod block_item;
mod storage;

pub trait ItemStack where Self:Any{
    fn render(&self,&GameData,&UiCore,&mut RenderBuffer2d,position:&Rectangle<f32>);
    fn stack_from(&mut self,&GameData,Box<ItemStack>)->Option<Box<ItemStack>>;
}