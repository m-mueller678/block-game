use glium::glutin::WindowEvent;
use glium::Frame;
use super::ui_core::UiCore;
pub use self::layer_controller::MenuLayerController;
pub use self::accessor::{Accessor, BoundedAccessor};

mod layer_controller;
mod items;
pub mod accessor;

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

use module::GameData;
use player::Player;
use std::sync::{Arc, Mutex};
use geometry::Rectangle;
use item::Slot;
use self::accessor::PlayerInventoryAccessor;
use self::items::InventoryUi;

pub struct TestMenu {
    inventory: InventoryUi<PlayerInventoryAccessor>,
}

impl TestMenu {
    const INVENTORY_AREA: Rectangle<f32> = Rectangle {
        min_y: 0.3,
        max_y: 0.6,
        min_x: 0.3,
        max_x: 0.7,
    };
    pub fn new(game_data: GameData, player: Arc<Mutex<Player>>) -> Self {
        println!("create test menu");
        TestMenu {
            inventory: InventoryUi::new(10, game_data, PlayerInventoryAccessor::new(player))
        }
    }
}

impl Menu for TestMenu {
    fn transparent(&self) -> bool { true }

    fn process_event(&mut self, e: &WindowEvent, ui_core: &mut UiCore) -> EventResult {
        use glium::glutin::*;
        match *e {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(VirtualKeyCode::Escape),
                    ..
                },
                ..
            } => {
                EventResult::MenuClosed
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                let pos = Self::INVENTORY_AREA.pos_to_local(ui_core.mouse_position);
                if pos.iter().all(|&x| x >= 0. && x <= 1.) {
                    let mut held_item = Slot::new();
                    self.inventory.click(pos[0], pos[1], &mut held_item);
                }
                EventResult::Processed
            }
            _ => EventResult::Processed
        }
    }

    fn render(&mut self, ui_core: &UiCore, target: &mut Frame) {
        use graphics::{RenderBuffer2d, VirtualDisplay};
        use glium::uniforms::SamplerWrapFunction;
        let mut render_buffer = RenderBuffer2d::new(&ui_core.display);
        {
            let mut inventory_display = render_buffer.sub_display(Self::INVENTORY_AREA);
            self.inventory.render(&mut inventory_display, ui_core);
        }
        let sampler = ui_core.textures.sampled().wrap_function(SamplerWrapFunction::Repeat);
        render_buffer.render(target, &ui_core.shader.tri_2d, sampler);
    }
}

impl Drop for TestMenu {
    fn drop(&mut self) {
        println!("test menu dropped")
    }
}