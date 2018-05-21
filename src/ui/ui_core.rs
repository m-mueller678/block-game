use glium::texture::CompressedSrgbTexture2dArray;
use block_texture_loader::TextureLoader;
use glium::backend::glutin::Display;
use glium_text_rusttype::TextSystem;
use glium::glutin::{MouseCursor, CursorState};
use graphics::*;
use super::KeyboardState;

const DISABLE_MOUSE_GRABBING: bool = true;

pub struct UiCore {
    pub display: Display,
    pub shader: Shader,
    pub textures: CompressedSrgbTexture2dArray,
    pub key_state: KeyboardState,
    pub mouse_position: [f32; 2],
    pub font_texture: FontTextureHandle,
    pub text_system: TextSystem,
    pub window_size: (u32, u32),

    pending_cursor_change: Option<CursorState>,
}

impl UiCore {
    pub fn new(display: Display, textures: TextureLoader) -> Self {
        let shader = match Shader::new(&display) {
            Ok(s) => s,
            Err(e) => {
                use std::process::exit;
                eprintln!("shader compilation failed:\n{}", e);
                exit(1)
            }
        };
        let window_size = display.gl_window().get_inner_size().unwrap();
        UiCore {
            shader: shader,
            textures: textures.load(&display),
            key_state: KeyboardState::new(),
            mouse_position: [0.5; 2],
            font_texture: FontTextureHandle::new(&display),
            text_system: TextSystem::new(&display),
            window_size,
            display,
            pending_cursor_change: None,
        }
    }

    pub fn update(&mut self) {
        if let Some(s) = self.pending_cursor_change {
            self.set_cursor_state(s);
        }
    }

    pub fn enable_cursor(&mut self) {
        if DISABLE_MOUSE_GRABBING {
            return;
        }
        self.set_cursor_state(CursorState::Normal);
        let win = self.display.gl_window();
        win.set_cursor(MouseCursor::Default);
    }

    pub fn disable_cursor(&mut self) {
        if DISABLE_MOUSE_GRABBING {
            return;
        }
        self.set_cursor_state(CursorState::Grab);
        let win = self.display.gl_window();
        win.set_cursor(MouseCursor::NoneCursor);
    }

    fn set_cursor_state(&mut self, state: CursorState) {
        let win = self.display.gl_window();
        if win.set_cursor_state(state).is_err() {
            self.pending_cursor_change = Some(state);
        } else {
            self.pending_cursor_change = None;
        }
    }
}
