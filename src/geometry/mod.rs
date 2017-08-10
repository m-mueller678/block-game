mod direction;
pub mod ray;

pub use self::direction::*;

pub struct Rectangle<T>{
    pub top:T,
    pub bottom:T,
    pub left:T,
    pub right:T,
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