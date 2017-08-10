use glium::texture::CompressedSrgbTexture2dArray;
use block_texture_loader::TextureLoader;
use glium::backend::glutin::Display;
use graphics::*;
use super::KeyboardState;


pub struct UiCore {
    pub display: Display,
    pub shader: Shader,
    pub textures: CompressedSrgbTexture2dArray,
    pub key_state: KeyboardState,
}

impl UiCore{
    pub fn new(display:Display, textures: TextureLoader) ->Self{
        let shader=match Shader::new(&display){
            Ok(s)=>s,
            Err(e)=>{
                use std::process::exit;
                eprintln!("shader compilation failed:\n{}",e);
                exit(1);
            }
        };
        UiCore{
            shader:shader,
            textures: textures.load(&display),
            display,
            key_state:KeyboardState::new(),
        }
    }
}