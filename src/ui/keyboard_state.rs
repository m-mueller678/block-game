use std::collections::hash_set::*;
use glium::glutin::{VirtualKeyCode,KeyboardInput,ElementState};

#[derive(Default)]
pub struct KeyboardState{
    pressed:HashSet<VirtualKeyCode>,
}

impl KeyboardState{
    pub fn new()->Self{
        Default::default()
    }
    pub fn update(&mut self,ki:&KeyboardInput){
        if let Some(code)=ki.virtual_keycode{
            match ki.state{
                ElementState::Pressed=>{self.pressed.insert(code);},
                ElementState::Released=>{self.pressed.remove(&code);},
            }
        }
    }
    pub fn pressed(&self,k:VirtualKeyCode)->bool{
        self.pressed.contains(&k)
    }
}