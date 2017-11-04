use glium::glutin::WindowEvent;
use glium::Frame;
use super::ui_core::UiCore;
pub use self::layer_controller::MenuLayerController;
pub use self::player_inventory::PlayerInventory;

mod layer_controller;
mod items;
mod player_inventory;

#[must_use]
pub enum EventResult {
    Processed,
    MenuClosed,
    NewMenu(Box<Menu>),
}

pub trait Menu {
    fn transparent(&self) -> bool;
    fn process_event(&mut self, event: &WindowEvent, ui_core: &mut UiCore) -> EventResult;
    fn render(&mut self, &UiCore, &mut Frame);
}
