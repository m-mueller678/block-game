use glium::texture::CompressedSrgbTexture2dArray;
use glium::backend::glutin::Display;
use graphics::*;
use module::StartComplete;
use super::KeyboardState;


pub struct UiCore {
    pub display: Display,
    pub shader: Shader,
    pub textures: CompressedSrgbTexture2dArray,
    pub key_state: KeyboardState,
}

impl UiCore{
    pub fn new(display:Display,start:StartComplete)->Self{
        UiCore{
            shader:Shader::new(&display).unwrap(),
            textures: start.textures.load(&display),
            display,
            key_state:KeyboardState::new(),
        }
    }
}