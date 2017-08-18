use std::rc::Rc;
use glium_text_rusttype::TextDisplay;
pub use self::render_buffer::{RenderBuffer2d, load_2d_shader};
use graphics::{TextureId, FontTextureHandle};
use geometry::Rectangle;

mod render_buffer;

/// Virtual display coordinates range fom 0 to 1
/// the origin is the top left corner of the display
pub trait VirtualDisplay {
    ///position in Virtual display coordinates
    fn textured_triangle(&mut self, position: [[f32; 2]; 3], tex_coords: [[f32; 2]; 3], texture_id: TextureId, brightness: f32);
    ///position in Virtual display coordinates
    fn textured_quad(&mut self, position: [[f32; 2]; 4], tex_coords: [[f32; 2]; 4], texture_id: TextureId, brightness: f32);
    fn fill_with_texture(&mut self, id: TextureId, brightness: f32) {
        self.textured_quad(
            [[0., 0.], [0., 1.], [1., 1.], [1., 0.]],
            [[0., 1.], [0., 0.], [1., 0.], [1., 1.]],
            id,
            brightness
        )
    }

    fn text(&mut self, Rc<TextDisplay<FontTextureHandle>>, pos: Rectangle<f32>);

    fn x_y_ratio(&self) -> f32;
    ///size in ui-units; one ui-unit is the size of an item slot
    fn ui_size_x(&self) -> f32;
    ///size in ui-units; one ui-unit is the size of an item slot
    fn ui_size_y(&self) -> f32;
    fn sub_display(&mut self, area: Rectangle<f32>) -> TransformedDisplay<Self> where Self: Sized {
        TransformedDisplay {
            mul_x: area.max_x - area.min_x,
            add_x: area.min_x,
            mul_y: area.max_y - area.min_y,
            add_y: area.min_y,
            display: self
        }
    }
}

pub struct TransformedDisplay<'a, D: 'a + VirtualDisplay> {
    mul_x: f32,
    add_x: f32,
    mul_y: f32,
    add_y: f32,
    display: &'a mut D
}

impl<'a, D: 'a + VirtualDisplay> TransformedDisplay<'a, D> {
    fn map(&self, pos: [f32; 2]) -> [f32; 2] {
        [
            self.map_x(pos[0]),
            self.map_y(pos[1]),
        ]
    }
    fn map_x(&self, x: f32) -> f32 {
        x.mul_add(self.mul_x, self.add_x)
    }
    fn map_y(&self, y: f32) -> f32 {
        y.mul_add(self.mul_y, self.add_y)
    }
}

impl<'a, D: 'a + VirtualDisplay> VirtualDisplay for TransformedDisplay<'a, D> {
    fn textured_triangle(&mut self, position: [[f32; 2]; 3], tex_coords: [[f32; 2]; 3], texture_id: TextureId, brightness: f32) {
        let position = [self.map(position[0]), self.map(position[1]), self.map(position[2]), ];
        self.display.textured_triangle(position, tex_coords, texture_id, brightness);
    }
    fn textured_quad(&mut self, position: [[f32; 2]; 4], tex_coords: [[f32; 2]; 4], texture_id: TextureId, brightness: f32) {
        let position = [
            self.map(position[0]),
            self.map(position[1]),
            self.map(position[2]),
            self.map(position[3]),
        ];
        self.display.textured_quad(position, tex_coords, texture_id, brightness);
    }
    fn text(&mut self, text: Rc<TextDisplay<FontTextureHandle>>, pos: Rectangle<f32>) {
        let rect = Rectangle {
            max_x: self.map_x(pos.max_x),
            min_x: self.map_x(pos.min_x),
            max_y: self.map_y(pos.max_y),
            min_y: self.map_y(pos.min_y),
        };
        self.display.text(text, rect)
    }
    fn x_y_ratio(&self) -> f32 {
        self.display.x_y_ratio() * self.mul_x / self.mul_y
    }
    fn ui_size_x(&self) -> f32 { self.display.ui_size_x() * self.mul_x }
    fn ui_size_y(&self) -> f32 { self.display.ui_size_y() * self.mul_y }
}