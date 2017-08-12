pub use self::render_buffer::{RenderBuffer2d, load_2d_shader};
use graphics::TextureId;
use geometry::Rectangle;

mod render_buffer;

/// Virtual display coordinates range fom 0 to 1
/// the origin is the top left corner of the display
pub trait VirtualDisplay {
    ///position in Virtual display coordinates
    fn textured_triangle(&mut self, position: [[f32; 2]; 3], tex_coords: [[f32; 2]; 3], texture_id: TextureId, brightness: f32);
    ///position in Virtual display coordinates
    fn textured_quad(&mut self, position: [[f32; 2]; 4], tex_coords: [[f32; 2]; 4], texture_id: TextureId, brightness: f32);
    fn x_y_ratio(&self) -> f32;
    ///size in ui-units; one ui-unit is the size of an item slot
    fn ui_size_x(&self) -> f32;
    ///size in ui-units; one ui-unit is the size of an item slot
    fn ui_size_y(&self) -> f32;
    fn sub_display(&mut self, area: Rectangle<f32>) -> TransformedDisplay<Self> where Self: Sized {
        TransformedDisplay {
            mul_x: area.right - area.left,
            add_x: area.left,
            mul_y: area.bottom - area.top,
            add_y: area.top,
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
            pos[0].mul_add(self.mul_x, self.add_x),
            pos[1].mul_add(self.mul_y, self.add_y),
        ]
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
    fn x_y_ratio(&self) -> f32 {
        self.display.x_y_ratio() * self.mul_x / self.mul_y
    }
    fn ui_size_x(&self) -> f32 { self.display.ui_size_x() * self.mul_x }
    fn ui_size_y(&self) -> f32 { self.display.ui_size_y() * self.mul_y }
}