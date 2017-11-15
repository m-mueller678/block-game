mod chunk_map;
mod chunk_loading;

pub mod random;
pub mod biome;
pub mod generator;
pub mod timekeeper;

pub use self::random::{WorldRngSeeder, WorldGenRng};
pub use self::chunk_map::*;
pub use self::chunk_loading::LoadGuard;
use std::ops::Deref;
use block::AtomicBlockId;
use std::sync::{Arc, Mutex, MutexGuard,Weak};
use self::chunk_loading::LoadMap;
use timekeeper::Timekeeper;
use module::GameData;

pub struct WorldReadGuard<'a>(&'a ChunkMap);

impl<'a> Deref for WorldReadGuard<'a>{
    type Target = ChunkMap;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}


pub type TimeGuard<'a> = MutexGuard<'a, Timekeeper>;

pub struct World {
    chunks: ChunkMap,
    inserter: PreInserter,
    loaded: LoadMap,
    game_data: GameData,
    time: Mutex<Timekeeper>,
}

impl World {
    pub fn new(game_data: GameData) -> Arc<Self> {
        let ret=Arc::new(World {
            chunks: ChunkMap::new(Arc::clone(&game_data)),
            inserter: PreInserter::new(Arc::clone(&game_data)),
            loaded: LoadMap::new(Weak::new()),
            game_data,
            time: Mutex::new(Timekeeper::new()),
        });
        ret.loaded.reset_world(Arc::downgrade(&ret));
        ret
    }

    pub fn time(&self) -> TimeGuard {
        self.time.lock().unwrap()
    }

    pub fn game_data(&self) -> &GameData {
        &self.game_data
    }

    pub fn read(&self) -> WorldReadGuard {
        WorldReadGuard(&self.chunks)
    }

    pub fn load_cube(&self, center: ChunkPos, radius: i32) -> LoadGuard {
        self.loaded.load_cube(center, radius)
    }
}
