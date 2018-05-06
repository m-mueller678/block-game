use std::ops::Deref;
use geometry::Direction;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Debug, Hash)]
pub struct ChunkPos(pub [i32; 3]);

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct BlockPos(pub [i32; 3]);

impl ChunkPos {
    pub fn facing(&self, d: Direction) -> Self {
        ChunkPos(d.apply_to_pos(self.0))
    }

    pub fn square_distance(&self, other: ChunkPos) -> i32 {
        use vecmath::{vec3_sub, vec3_square_len};
        vec3_square_len(vec3_sub(**self, *other))
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
