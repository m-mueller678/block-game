mod direction;

use num::Float;

pub mod ray;

pub use self::direction::*;

#[derive(Clone, Copy, Debug)]
pub struct Rectangle<T> {
    pub min_y: T,
    pub max_y: T,
    pub min_x: T,
    pub max_x: T,
}

impl<T: Float> Rectangle<T> {
    pub fn pos_to_local(&self, pos: [T; 2]) -> [T; 2] {
        [
            (pos[0] - self.min_x) / (self.max_x - self.min_x),
            (pos[1] - self.min_y) / (self.max_y - self.min_y),
        ]
    }
}

pub const CORNER_OFFSET: [[f32; 3]; 8] = [
    [1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.],
    [1., 0., 1.], [0., 0., 1.], [0., 1., 1.], [1., 1., 1.],
];

pub const CUBE_FACES: [[usize; 4]; 6] = [
    [0, 4, 7, 3],
    [5, 1, 2, 6],
    [6, 2, 3, 7],
    [1, 5, 4, 0],
    [4, 5, 6, 7],
    [1, 0, 3, 2],
];