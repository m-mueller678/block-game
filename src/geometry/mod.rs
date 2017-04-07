mod direction;
pub mod ray;

pub use self::direction::*;

pub const CORNER_OFFSET: [[f32; 3]; 8] = [
    [1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.],
    [1., 0., 1.], [0., 0., 1.], [0., 1., 1.], [1., 1., 1.],
];

pub const CUBE_FACES: [[usize; 4]; 6] = [
    [0, 4, 7, 3],
    [1, 2, 6, 5],
    [2, 3, 7, 6],
    [0, 1, 5, 4],
    [4, 5, 6, 7],
    [3, 2, 1, 0],
];