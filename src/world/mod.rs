mod chunk_map;
mod chunk_loading;

pub mod random;
pub mod biome;
pub mod generator;
pub mod timekeeper;

pub use self::random::{WorldRngSeeder,WorldGenRng};
pub use self::chunk_map::*;
pub use self::chunk_loading::LoadGuard;
use block::AtomicBlockId;
use std::sync::{Arc, RwLock, RwLockReadGuard,Mutex,MutexGuard};
use self::chunk_loading::LoadMap;
use timekeeper::Timekeeper;
use module::GameData;

pub type WorldReadGuard<'a> = RwLockReadGuard<'a, ChunkMap>;
pub type TimeGuard<'a>=MutexGuard<'a,Timekeeper>;

pub struct World {
    chunks: RwLock<ChunkMap>,
    inserter: Inserter,
    loaded: LoadMap,
    game_data: GameData,
    time: Mutex<Timekeeper>,
}

impl World {
    pub fn new(game_data:GameData) -> Self {
        World {
            chunks: RwLock::new(ChunkMap::new(Arc::clone(&game_data))),
            inserter: Inserter::new(Arc::clone(&game_data)),
            loaded: LoadMap::new(),
            game_data,
            time:Mutex::new(Timekeeper::new()),
        }
    }

    pub fn time(&self)->TimeGuard{
        self.time.lock().unwrap()
    }

    pub fn game_data(&self)->&GameData{
        &self.game_data
    }

    pub fn read(&self) -> WorldReadGuard {
        self.chunks.read().unwrap()
    }

    pub fn load_cube(&self, center: ChunkPos, radius: i32) -> LoadGuard {
        self.loaded.load_cube(center, radius)
    }

    pub fn flush_chunk(&self) {
        let mut chunk_lock = self.chunks.write().unwrap();
        self.loaded.apply_to_world(&mut *chunk_lock, &self.inserter);
    }
}
