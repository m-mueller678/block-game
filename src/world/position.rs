use geometry::Direction;
use std::ops::Index;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct ChunkPos(pub [i32; 3]);

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct BlockPos(pub [i32; 3]);

impl ChunkPos {
    pub fn facing(&self, d: Direction) -> Self {
        ChunkPos(d.apply_to_pos(self.0))
    }
}

impl BlockPos {
    pub fn facing(&self, d: Direction) -> Self {
        BlockPos(d.apply_to_pos(self.0))
    }
}

impl Index<usize> for ChunkPos {
    type Output = i32;
    fn index(&self, i: usize) -> &i32 {
        &self.0[i]
    }
}

impl Index<usize> for BlockPos {
    type Output = i32;
    fn index(&self, i: usize) -> &i32 {
        &self.0[i]
    }
}
