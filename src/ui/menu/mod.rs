use glium::glutin::WindowEvent;
use glium::Frame;
use super::ui_core::UiCore;
pub use self::layer_controller::MenuLayerController;

mod layer_controller;

#[must_use]
pub enum EventResult {
    Processed,
    MenuClosed,
    NewMenu(Box<Menu>),
}

pub trait Menu {
    fn transparent(&self) -> bool;
    fn process_event(&mut self, event: &WindowEvent, ui_core: &mut UiCore) -> EventResult;
    fn render(&mut self, &UiCore,&mut Frame);
}


use module::GameData;

pub struct TestMenu{
    game_data:GameData
}

impl TestMenu{
    pub fn new(game_data:GameData)->Self{
        println!("create test menu");
        TestMenu{game_data}
    }
}

impl Menu for TestMenu{
    fn transparent(&self)->bool{true}
    fn process_event(&mut self,e:&WindowEvent,_:&mut UiCore)->EventResult{
        use glium::glutin::*;
        if let WindowEvent::KeyboardInput {input:KeyboardInput{state:ElementState::Pressed,virtual_keycode,..},..}=*e{
            match virtual_keycode{
                Some(VirtualKeyCode::Escape)=>{
                    EventResult::MenuClosed
                },
                Some(VirtualKeyCode::I)=>{
                    EventResult::NewMenu(Box::new(TestMenu::new(self.game_data.clone())))
                },
                _=>EventResult::Processed,
            }
        }else{
            EventResult::Processed
        }
    }
    fn render(&mut self, ui_core: &UiCore,target:&mut Frame) {
        use item::{BlockItem,ItemStack};
        use geometry::Rectangle;
        use graphics::RenderBuffer2d;
        use glium::uniforms::SamplerWrapFunction;
        let item=BlockItem::new(self.game_data.blocks().by_name("grass").unwrap(),1);
        let mut render_buffer=RenderBuffer2d::new();
        item.render(&self.game_data,ui_core,&mut render_buffer,&Rectangle{top:0.7,bottom:0.3,left:0.3,right:0.7});
        let sampler = ui_core.textures.sampled().wrap_function(SamplerWrapFunction::Repeat);
        render_buffer.render(target,&ui_core.shader.tri_2d,sampler,&ui_core.display);
    }
}

impl Drop for TestMenu{
    fn drop(&mut self) {
        println!("test menu dropped")
    }
}