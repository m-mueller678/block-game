#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    PosX = 0,
    NegX = 1,
    PosY = 2,
    NegY = 3,
    PosZ = 4,
    NegZ = 5
}

pub const ALL_DIRECTIONS: [Direction; 6] = [
    Direction::PosX,
    Direction::NegX,
    Direction::PosY,
    Direction::NegY,
    Direction::PosZ,
    Direction::NegZ,
];

impl Direction {
    pub fn from_usize(i: usize) -> Self {
        assert!(i < 6, "invalid direction id");
        ALL_DIRECTIONS[i]
    }
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
    pub fn invert(self) -> Self {
        match self {
            Direction::PosX => Direction::NegX,
            Direction::NegX => Direction::PosX,
            Direction::PosY => Direction::NegY,
            Direction::NegY => Direction::PosY,
            Direction::PosZ => Direction::NegZ,
            Direction::NegZ => Direction::PosZ,
        }
    }
}
