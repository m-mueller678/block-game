mod chunk_map;
mod chunk_loading;

pub mod random;
pub mod biome;
pub mod generator;

pub use self::random::{WorldRngSeeder,WorldGenRng};
pub use self::chunk_map::*;
pub use self::chunk_loading::LoadGuard;
use block::AtomicBlockId;
use biome::BiomeId;
use block::BlockRegistry;
use std::sync::{Arc, RwLock, RwLockReadGuard, Mutex};
use self::chunk_loading::LoadMap;
use self::generator::Generator;

pub type WorldReadGuard<'a> = RwLockReadGuard<'a, ChunkMap>;

pub struct World {
    chunks: RwLock<ChunkMap>,
    inserter: Inserter,
    loaded: LoadMap,
}

impl World {
    pub fn new(blocks: Arc<BlockRegistry>, gen: Box<Generator>) -> Self {
        World {
            chunks: RwLock::new(ChunkMap::new(blocks)),
            inserter: Inserter::new(gen),
            loaded: LoadMap::new(),
        }
    }

    pub fn generator(&self)->&Generator{
        self.inserter.generator()
    }

    pub fn read(&self) -> WorldReadGuard {
        self.chunks.read().unwrap()
    }

    pub fn load_cube(&self, center: &ChunkPos, radius: i32) -> LoadGuard {
        self.loaded.load_cube(center, radius)
    }

    pub fn flush_chunk(&self) {
        let mut chunk_lock = self.chunks.write().unwrap();
        self.loaded.apply_to_world(&mut *chunk_lock, &self.inserter);
    }
}
