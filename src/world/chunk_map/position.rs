use std::ops::Deref;
use num::Integer;
use world::CHUNK_SIZE;
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

    pub fn containing_chunk(&self)->ChunkPos{
        static CS:i32=CHUNK_SIZE as i32;
        ChunkPos([
            self[0].div_floor(&CS),
            self[1].div_floor(&CS),
            self[2].div_floor(&CS),
        ])
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
