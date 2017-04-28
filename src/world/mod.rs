mod random;
mod generator;
mod chunk_map;

pub use self::random::WorldRngSeeder;
pub use self::chunk_map::*;
pub use self::generator::{Generator, ParameterWeight, WorldGenBlock, EnvironmentData};

use block::BlockRegistry;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::ops::Deref;

pub type WorldReadGuard<'a> = RwLockReadGuard<'a, ChunkMap>;
pub type WorldWriteGuard<'a> = RwLockWriteGuard<'a, ChunkMap>;

pub fn new_world(blocks: Arc<BlockRegistry>, generator: Generator) -> (WorldReader, WorldWriter) {
    let chunk_map = Arc::new(RwLock::new(ChunkMap::new(blocks)));
    let cm2 = chunk_map.clone();
    (WorldReader { chunks: cm2, env_data: generator.env_data().clone() }, WorldWriter {
        reader: WorldReader { chunks: chunk_map, env_data: generator.env_data().clone() },
        inserter: Inserter::new(generator)
    })
}

#[derive(Clone)]
pub struct WorldReader {
    env_data: EnvironmentData,
    chunks: Arc<RwLock<ChunkMap>>,
}

impl WorldReader {
    pub fn read(&self) -> WorldReadGuard {
        self.chunks.read().unwrap()
    }
    pub fn env_data(&self) -> &EnvironmentData {
        &self.env_data
    }
}

pub struct WorldWriter {
    reader: WorldReader,
    inserter: Inserter,
}

impl Deref for WorldWriter {
    type Target = WorldReader;
    fn deref(&self) -> &WorldReader {
        &self.reader
    }
}

impl WorldWriter {
    pub fn gen_area(&mut self, pos: &BlockPos, range: i32) {
        let base = chunk_at(pos);
        for x in (base[0] - range)..(base[0] + range + 1) {
            for y in (base[1] - range)..(base[1] + range + 1) {
                for z in (base[2] - range)..(base[2] + range + 1) {
                    self.inserter.insert(&ChunkPos([x, y, z]), &self.reader.read());
                }
            }
        }
    }
    pub fn flush_chunk(&mut self) {
        self.inserter.push_to_world(&mut self.reader.chunks.write().unwrap());
    }
}

