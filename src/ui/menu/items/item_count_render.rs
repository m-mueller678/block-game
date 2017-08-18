use std::rc::Rc;
use glium_text_rusttype::TextDisplay;
use graphics::{VirtualDisplay, FontTextureHandle};
use ui::UiCore;
use geometry::Rectangle;

pub struct ItemCountRender {
    count: usize,
    text: Rc<TextDisplay<FontTextureHandle>>,
    area: Rectangle<f32>,
}

impl ItemCountRender {
    pub fn new(core: &UiCore) -> Self {
        ItemCountRender {
            count: 0,
            text: Rc::new(TextDisplay::new(&core.text_system, core.font_texture.clone(), "")),
            area: Rectangle {
                min_y: 0.,
                max_y: 1.,
                min_x: 0.,
                max_x: 1.,
            },
        }
    }

    pub fn render<V:VirtualDisplay>(&self, display:&mut V) {
        display.text(self.text.clone(),self.area)
    }

    pub fn update(&mut self, count: usize) {
        if count==self.count{
            return;
        }
        self.count = count;
        Rc::get_mut(&mut self.text)
            .expect("TextDisplay owned by ItemCountRender was not returned before update")
            .set_text(&format!("{}", count));
        let ratio = self.text.get_width() / self.text.get_height();
        self.area = if ratio > 5. {
            Rectangle {
                min_y: 0.75,
                max_y: 0.9 / ratio,
                min_x: 0.05,
                max_x: 0.95
            }
        } else {
            Rectangle {
                min_y: 0.75,
                max_y: 0.95,
                min_x: 0.95 - 0.2 * ratio,
                max_x: 0.95
            }
        }
    }
}