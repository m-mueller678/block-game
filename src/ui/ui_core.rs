use glium::texture::CompressedSrgbTexture2dArray;
use block_texture_loader::TextureLoader;
use glium::backend::glutin::Display;
use glium_text_rusttype::TextSystem;
use graphics::*;
use super::KeyboardState;


pub struct UiCore {
    pub display: Display,
    pub shader: Shader,
    pub textures: CompressedSrgbTexture2dArray,
    pub key_state: KeyboardState,
    pub mouse_position: [f32; 2],
    pub font_texture: FontTextureHandle,
    pub text_system: TextSystem,
}

impl UiCore {
    pub fn new(display: Display, textures: TextureLoader) -> Self {
        let shader = match Shader::new(&display) {
            Ok(s) => s,
            Err(e) => {
                use std::process::exit;
                eprintln!("shader compilation failed:\n{}",e);
                exit(1)
            }
        };
        UiCore {
            shader: shader,
            textures: textures.load(&display),
            key_state: KeyboardState::new(),
            mouse_position: [0.5; 2],
            font_texture: FontTextureHandle::new(&display),
            text_system: TextSystem::new(&display),
            display,
        }
    }
}