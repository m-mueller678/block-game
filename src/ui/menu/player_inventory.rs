use owning_ref::ArcRef;
use glium::glutin::WindowEvent;
use glium::Frame;
use ui::ui_core::UiCore;
use module::GameData;
use player::Player;
use std::sync::Arc;
use geometry::Rectangle;
use item::*;
use super::items::InventoryUi;
use super::{Menu, EventResult};

pub struct PlayerInventory {
    inventory: InventoryUi<ArcRef<Player,SlotStorage>>,
    move_slot: Slot,
    area: Rectangle<f32>,
}

impl PlayerInventory {
    pub fn new(game_data: GameData, player: Arc<Player>) -> Self {
        let stack = Box::new(BlockItem::new(game_data.blocks().by_name("grass").unwrap(), 50));
        PlayerInventory {
            inventory: InventoryUi::new(10, game_data, ArcRef::new(player).map(|p|p.inventory())),
            move_slot: (stack as Box<ItemStack>).into(),
            area: Rectangle {
                min_y: 0.,
                max_y: 0.01,
                min_x: 0.,
                max_x: 0.01,
            }
        }
    }
}

impl Menu for PlayerInventory {
    fn transparent(&self) -> bool { true }

    fn process_event(&mut self, e: &WindowEvent, ui_core: &mut UiCore) -> EventResult {
        use glium::glutin::*;
        match *e {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: code,
                    ..
                },
                ..
            } if code == Some(VirtualKeyCode::I) || code == Some(VirtualKeyCode::Escape) => {
                EventResult::MenuClosed
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                let pos = self.area.pos_to_local(ui_core.mouse_position);
                if pos.iter().all(|&x| x >= 0. && x <= 1.) {
                    self.inventory.click(pos[0], pos[1], &mut self.move_slot);
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
            let inv_size = self.inventory.size();
            let hw = (inv_size.0 / render_buffer.ui_size_x()).min(0.5);
            let hh = (inv_size.1 / render_buffer.ui_size_y()).min(0.5);
            self.area = Rectangle {
                min_y: 0.5 - hh,
                max_y: 0.5 + hh,
                min_x: 0.5 - hw,
                max_x: 0.5 + hw,
            };
            let mut inventory_display = render_buffer.sub_display(self.area);
            self.inventory.render(&mut inventory_display, ui_core);
        }
        let sampler = ui_core.textures.sampled().wrap_function(SamplerWrapFunction::Repeat);
        render_buffer.render(target, &ui_core.shader.tri_2d, sampler, &ui_core.text_system);
    }
}