use glium::glutin::WindowEvent;
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
    fn render(&mut self, &mut UiCore);
}


pub struct TestMenu{}

impl TestMenu{
    pub fn new()->Self{
        println!("create test menu");
        TestMenu{}
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
                    EventResult::NewMenu(Box::new(TestMenu::new()))
                },
                _=>EventResult::Processed,
            }
        }else{
            EventResult::Processed
        }
    }
    fn render(&mut self, _: &mut UiCore) {

    }
}

impl Drop for TestMenu{
    fn drop(&mut self) {
        println!("test menu dropped")
    }
}