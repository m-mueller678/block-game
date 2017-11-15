use std;
use chashmap::*;
use std::sync::atomic::Ordering;
use num::Integer;
use crossbeam::sync::SegQueue;
use block::BlockId;
use geometry::ray::{Ray, BlockIntersection};
use module::GameData;

mod pre_inserter;
mod position;
mod chunk_loader;
mod lighting;
mod atomic_light;
mod chunk;

pub use self::position::*;
pub use self::pre_inserter::PreInserter;
use self::lighting::LightUpdater;
use self::chunk_loader::ChunkLoader;
pub use self::chunk::*;

use self::lighting::*;

pub struct ChunkMap {
    chunks: CHashMap<[i32; 3], Box<Chunk>>,
    game_data: GameData,
    chunk_updates: SegQueue<ChunkPos>,
    chunk_loader: ChunkLoader,
    light_updater: LightUpdater,
}

impl ChunkMap {
    pub fn new(game_data: GameData) -> Self {
        ChunkMap {
            chunks: CHashMap::new(),
            game_data: game_data,
            chunk_updates: SegQueue::new(),
            chunk_loader: ChunkLoader::new(),
            light_updater: LightUpdater::new(),
        }
    }
    pub fn set_block(&self, pos: BlockPos, block: BlockId) -> Result<(), ()> {
        let chunk_pos = Self::chunk_at(pos);
        if let Some(chunk) = self.borrow_chunk(chunk_pos) {
            let before;
            {
                before = chunk.data[pos].load();
                chunk.data[pos].store(block);
            }
            let before_lt = self.game_data.blocks().light_type(before);
            let new_lt = self.game_data.blocks().light_type(block);
            if *before_lt != *new_lt {
                let light = &chunk.artificial_light[pos];
                self.light_updater.block_light_changed((light.level(), light.direction()), new_lt, pos);
            }
            Self::set_chunk_update(&self.chunk_updates, &*chunk, chunk_pos);
            if self.game_data.blocks().is_opaque_draw(before) !=
                self.game_data.blocks().is_opaque_draw(block) {
                self.update_adjacent_chunks(pos);
            }
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn poll_chunk_update(&self) -> Option<ChunkPos> {
        while let Some(pos) = self.chunk_updates.try_pop() {
            let still_needs_update = self.borrow_chunk(pos)
                .map(|c| c.update_render.swap(false, Ordering::Acquire))
                .unwrap_or(false);
            if still_needs_update {
                return Some(pos);
            }
        }
        None
    }
    pub fn chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.borrow_chunk(pos).is_some()
    }
    pub fn chunk_at(pos: BlockPos) -> ChunkPos {
        use num::Integer;
        ChunkPos([pos[0].div_floor(&(CHUNK_SIZE as i32)),
            pos[1].div_floor(&(CHUNK_SIZE as i32)),
            pos[2].div_floor(&(CHUNK_SIZE as i32))])
    }
    pub fn game_data(&self) -> &GameData {
        &self.game_data
    }
    pub fn light(&self) -> &LightUpdater {
        &self.light_updater
    }
    pub fn chunk_loader(&self) -> &ChunkLoader {
        &self.chunk_loader
    }

    pub fn complete_tick(&self) {
        self.chunk_loader.apply(self);
        self.light_updater.apply(self);
    }

    pub fn lock_chunk(&self, pos: ChunkPos) -> Option<ChunkReader> {
        self.borrow_chunk(pos).map(ChunkReader::new)
    }
    pub fn block_ray_trace(&self,
                           start: [f32; 3],
                           direction: [f32; 3],
                           range: f32)
                           -> Option<BlockIntersection> {
        for intersect in Ray::new(start, direction).blocks() {
            let sq_dist: f32 = intersect
                .block
                .0
                .iter()
                .map(|x| *x as f32 + 0.5)
                .zip(start.iter())
                .map(|x| x.1 - x.0)
                .map(|x| x * x)
                .sum();
            if sq_dist > range * range {
                return None;
            }
            if let Some(id) = self.get_block(intersect.block) {
                if self.game_data.blocks().is_opaque_draw(id) {
                    return Some(intersect);
                }
            }
        }
        unreachable!() // ray block iterator is infinite
    }
    pub fn get_block(&self, pos: BlockPos) -> Option<BlockId> {
        self.borrow_chunk(Self::chunk_at(pos))
            .map(|c| c.data[pos].load())
    }
    pub fn artificial_light(&self, pos: BlockPos) -> Option<(u8, LightDirection)> {
        if let Some(chunk) = self.borrow_chunk(Self::chunk_at(pos)) {
            let light = &chunk.artificial_light[pos];
            Some((light.level(), light.direction()))
        } else {
            None
        }
    }
    fn update_adjacent_chunks(&self, block_pos: BlockPos) {
        let cs = CHUNK_SIZE as i32;
        let chunk_pos = Self::chunk_at(block_pos);
        let (x, y, z) = (chunk_pos[0], chunk_pos[1], chunk_pos[2]);
        if block_pos[0].mod_floor(&cs) == 0 {
            self.update_render(ChunkPos([x - 1, y, z]))
        }
        if block_pos[1].mod_floor(&cs) == 0 {
            self.update_render(ChunkPos([x, y - 1, z]))
        }
        if block_pos[2].mod_floor(&cs) == 0 {
            self.update_render(ChunkPos([x, y, z - 1]))
        }
        if block_pos[0].mod_floor(&cs) == cs - 1 {
            self.update_render(ChunkPos([x + 1, y, z]))
        }
        if block_pos[1].mod_floor(&cs) == cs - 1 {
            self.update_render(ChunkPos([x, y + 1, z]))
        }
        if block_pos[2].mod_floor(&cs) == cs - 1 {
            self.update_render(ChunkPos([x, y, z + 1]))
        }
    }
    fn update_render(&self, pos: ChunkPos) {
        if let Some(chunk) = self.borrow_chunk(pos) {
            Self::set_chunk_update(&self.chunk_updates, &*chunk, pos);
        }
    }
    fn borrow_chunk(&self, p: ChunkPos) -> Option<ReadGuard<[i32; 3], Box<Chunk>>> {
        self.chunks.get(&[p[0], p[1], p[2]])
    }

    fn set_chunk_update(queue: &SegQueue<ChunkPos>, chunk: &Chunk, pos: ChunkPos) {
        let old = chunk.update_render.swap(true, Ordering::Release);
        if old != true {
            queue.push(pos);
        }
    }
}

pub fn chunk_at(pos: BlockPos) -> ChunkPos {
    ChunkMap::chunk_at(pos)
}

pub struct ChunkReader<'a> {
    chunk: ReadGuard<'a, [i32; 3], Box<Chunk>>,
}

impl<'a> ChunkReader<'a> {
    pub fn new(chunk: ReadGuard<'a, [i32; 3], Box<Chunk>>) -> Self {
        ChunkReader { chunk: chunk }
    }
    pub fn block(&self, pos: [usize; 3]) -> BlockId {
        self.chunk.data[pos].load()
    }
    pub fn effective_light(&self, pos: [usize; 3]) -> u8 {
        self.chunk.artificial_light[pos].level()
    }
}

impl<'a> std::ops::Deref for ChunkReader<'a> {
    type Target = Chunk;
    fn deref(&self) -> &Self::Target {
        &*self.chunk
    }
}

pub struct ChunkCache<'a> {
    pos: ChunkPos,
    chunk: Option<ChunkReader<'a>>,
}

impl<'a> ChunkCache<'a> {
    pub fn new() -> Self {
        ChunkCache {
            pos: ChunkPos([0; 3]),
            chunk: None,
        }
    }

    pub fn load(&mut self, pos: ChunkPos, chunks: &'a ChunkMap) -> Result<(), ()> {
        if self.chunk.is_none() || pos != self.pos {
            if let Some(cref) = chunks.lock_chunk(pos) {
                self.pos = pos;
                self.chunk = Some(cref);
                Ok(())
            } else {
                Err(())
            }
        } else {
            Ok(())
        }
    }

    pub fn chunk(&self) -> &Chunk {
        self.chunk.as_ref().unwrap()
    }
}
