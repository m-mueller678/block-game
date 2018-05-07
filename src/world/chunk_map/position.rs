use std::ops::Deref;
use geometry::Direction;
use num::Integer;
use world::CHUNK_SIZE;

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

    pub fn pos_in_chunk(&self) -> (ChunkPos, [usize; 3]) {
        let cs = CHUNK_SIZE as i32;
        let (xq, xr) = self[0].div_mod_floor(&cs);
        let (yq, yr) = self[1].div_mod_floor(&cs);
        let (zq, zr) = self[2].div_mod_floor(&cs);
        (ChunkPos([xq, yq, zq]), [xr as usize, yr as usize, zr as usize])
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
