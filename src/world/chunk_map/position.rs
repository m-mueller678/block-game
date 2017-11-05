use std::ops::Deref;
use geometry::Direction;

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
pub struct ChunkPos(pub [i32; 3]);

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
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

impl Deref for ChunkPos {
    type Target = [i32; 3];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for BlockPos {
    type Target = [i32; 3];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
