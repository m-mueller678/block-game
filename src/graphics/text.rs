use std::ops::Deref;
use std::rc::Rc;
use glium::backend::Facade;
use glium_text_rusttype::FontTexture;
use font_loader::system_fonts;

#[derive(Clone)]
pub struct FontTextureHandle {
    texture: Rc<FontTexture>,
}

impl Deref for FontTextureHandle {
    type Target = FontTexture;

    fn deref(&self) -> &Self::Target {
        &*self.texture
    }
}

impl FontTextureHandle {
    pub fn new<F: Facade>(facade: &F) -> Self {
        let font_data = system_fonts::get(&system_fonts::FontPropertyBuilder::new().build())
            .expect("cannot find any system fonts").0;
        let character_list = (0x21u8..0x7F).map(|i| i as char);
        let font_texture = FontTexture::new(facade, &font_data as &[u8], 32, character_list)
            .expect("cannot rasterize font");

        FontTextureHandle {
            texture: Rc::new(font_texture)
        }
    }
}