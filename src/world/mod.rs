mod chunk_map;
mod chunk_loading;
mod inserter;
mod tick_executor;

pub mod random;
pub mod biome;
pub mod generator;
pub mod timekeeper;
pub mod block_controller;

pub use self::random::{WorldRngSeeder, WorldGenRng};
pub use self::chunk_map::{ChunkPos, Chunk, CHUNK_SIZE, BlockPos, chunk_at, ChunkArray};
pub use self::chunk_loading::LoadGuard;
pub use self::block_controller::{CreateError, BlockController};
pub use self::tick_executor::{TickFunction, TickFunctionResult};

use block::AtomicBlockId;
use std::sync::{Arc, Mutex, MutexGuard};
use self::chunk_loading::LoadMap;
use timekeeper::Timekeeper;
use module::GameData;
use graphics::ChunkUpdateSender;
use block::BlockId;
use geometry::Direction;
use self::chunk_map::{ChunkMap};
use self::inserter::Inserter;
use self::block_controller::BlockControllerMap;
use self::tick_executor::TickExecutor;

pub type TimeGuard<'a> = MutexGuard<'a, Timekeeper>;

pub struct World {
    chunks: ChunkMap,
    block_controllers: BlockControllerMap,
    inserter: Inserter,
    loaded: LoadMap,
    game_data: GameData,
    time: Mutex<Timekeeper>,
    tick_executor: TickExecutor,
}

impl World {
    pub fn new(game_data: GameData, chunk_sender: ChunkUpdateSender) -> Self {
        World {
            chunks: ChunkMap::new(Arc::clone(&game_data), chunk_sender),
            block_controllers: BlockControllerMap::new(),
            inserter: Inserter::new(Arc::clone(&game_data)),
            loaded: LoadMap::new(),
            game_data,
            time: Mutex::new(Timekeeper::new()),
            tick_executor: TickExecutor::new(),
        }
    }

    pub fn time(&self) -> TimeGuard {
        self.time.lock().unwrap()
    }

    pub fn game_data(&self) -> &GameData {
        &self.game_data
    }

    pub fn load_cube(&self, center: ChunkPos, radius: i32) -> LoadGuard {
        self.loaded.load_cube(center, radius)
    }

    pub fn set_block(&self, pos: BlockPos, block: BlockId) -> Result<(), ()> {
        self.chunks.set_block(pos, block)
    }

    pub fn get_block(&self, pos: BlockPos) -> Option<BlockId> {
        self.chunks.get_block(pos)
    }

    pub fn natural_light(&self, pos: BlockPos) -> Option<(u8, Option<Direction>)> {
        self.chunks.natural_light(pos)
    }

    pub fn artificial_light(&self, pos: BlockPos) -> Option<(u8, Option<Direction>)> {
        self.chunks.artificial_light(pos)
    }

    pub fn flush_chunk(&self) {
        self.loaded.apply_to_world(&self);
    }

    pub fn run_tick(&self) {
        let now = self.time().current_tick();
        self.tick_executor.run(&self, now);
    }

    pub fn on_tick(&self, f: TickFunction) {
        self.tick_executor.add(f.into());
    }
}
