use chashmap::ReadGuard;
use std::sync::Arc;
use super::{Chunk, ChunkPos, ChunkMap};

pub struct ChunkCache<'a> {
    pos: ChunkPos,
    chunk: ReadGuard<'a, [i32; 3], Arc<Chunk>>,
}

impl<'a> ChunkCache<'a> {
    pub fn new(pos: ChunkPos, chunks: &'a ChunkMap) -> Result<Self, ()> {
        if let Some(cref) = chunks.borrow_chunk(pos) {
            Ok(ChunkCache {
                pos: pos,
                chunk: cref,
            })
        } else {
            Err(())
        }
    }
    pub fn chunk(&self) -> &Chunk {
        &**self.chunk
    }
    pub fn load(&mut self, pos: ChunkPos, chunks: &'a ChunkMap) -> Result<(), ()> {
        if pos == self.pos {
            Ok(())
        } else {
            *self = Self::new(pos, chunks)?;
            Ok(())
        }
    }
    pub fn pos(&self) -> ChunkPos {
        self.pos
    }
}
