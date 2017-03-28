#[derive(Copy, Clone, Debug)]
pub enum Direction {
    PosX = 0,
    NegX = 1,
    PosY = 2,
    NegY = 3,
    PosZ = 4,
    NegZ = 5
}

impl Direction {
    pub fn offset(&self) -> [i32; 3] {
        match *self {
            Direction::PosX => [1, 0, 0],
            Direction::PosY => [0, 1, 0],
            Direction::PosZ => [0, 0, 1],
            Direction::NegX => [-1, 0, 0],
            Direction::NegY => [0, -1, 0],
            Direction::NegZ => [0, 0, -1],
        }
    }
    pub fn apply_to_pos(&self, pos: [i32; 3]) -> [i32; 3] {
        let offset = self.offset();
        [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
    }
}
