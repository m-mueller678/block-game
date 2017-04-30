mod random;
mod generator;
mod chunk_map;
mod chunk_loading;

pub use self::random::WorldRngSeeder;
pub use self::chunk_map::*;
pub use self::generator::{Generator, ParameterWeight, WorldGenBlock, EnvironmentData,structure};
pub use self::chunk_loading::LoadGuard;

use block::BlockRegistry;
use std::sync::{Arc, RwLock, RwLockReadGuard, Mutex};
use self::chunk_loading::LoadMap;

pub type WorldReadGuard<'a> = RwLockReadGuard<'a, ChunkMap>;

pub struct World {
    env_data: EnvironmentData,
    chunks: RwLock<ChunkMap>,
    inserter: Mutex<Inserter>,
    loaded: LoadMap,
}

impl World {
    pub fn new(blocks: Arc<BlockRegistry>, gen: Generator) -> Self {
        World {
            env_data: gen.env_data().clone(),
            chunks: RwLock::new(ChunkMap::new(blocks)),
            inserter: Mutex::new(Inserter::new(gen)),
            loaded: LoadMap::new(),
        }
    }

    pub fn read(&self) -> WorldReadGuard {
        self.chunks.read().unwrap()
    }

    pub fn load_cube(&self, center: &ChunkPos, radius: i32) -> LoadGuard {
        self.loaded.load_cube(center, radius)
    }

    pub fn env_data(&self) -> &EnvironmentData {
        &self.env_data
    }

    pub fn flush_chunk(&self) {
        let mut chunk_lock = self.chunks.write().unwrap();
        let mut inserter_lock = self.inserter.lock().unwrap();
        self.loaded.apply_to_world(&mut *chunk_lock, &mut *inserter_lock);
    }
}
