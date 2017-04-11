mod generator;
mod atomic_light;
mod chunk;
mod lighting;
mod position;

pub const MAX_NATURAL_LIGHT: u8 = 5;

pub use self::chunk::{ChunkReader, chunk_index, chunk_index_global, CHUNK_SIZE};
pub use self::generator::Generator;
pub use self::position::*;

use block::{AtomicBlockId, BlockId, BlockRegistry, LightType};
use self::lighting::*;
use std::collections::hash_map::HashMap;
use geometry::{Direction, ALL_DIRECTIONS};
use geometry::ray::{Ray, BlockIntersection};
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use num::Integer;
use self::atomic_light::{LightState};
use self::chunk::Chunk;

struct QueuedChunk {
    light_sources: Vec<(BlockPos, u8)>,
    pos: ChunkPos,
    data: [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

#[derive(Debug)]
pub enum ChunkAccessError {
    NoChunk
}

struct ChunkColumn {
    chunks: [Vec<Option<Chunk>>; 2]
}

impl ChunkColumn {
    fn new() -> Self {
        ChunkColumn {
            chunks: [Vec::new(), Vec::new()],
        }
    }
    fn get(&self, y: i32) -> Option<&Chunk> {
        let pos = y >= 0;
        let index = if pos {
            y as usize
        } else {
            (-y - 1) as usize
        };
        self.chunks[pos as usize].get(index).map(|o| o.as_ref()).unwrap_or(None)
    }
    fn insert(&mut self, y: i32, chunk: Chunk) -> &mut Chunk {
        let pos = y >= 0;
        let index = if pos {
            y as usize
        } else {
            (-y - 1) as usize
        };
        let vec = &mut self.chunks[pos as usize];
        if index >= vec.len() {
            let new_len = index + 1;
            let len_dif = new_len - vec.len();
            vec.reserve(len_dif);
            for _ in 0..(len_dif - 1) {
                vec.push(None);
            }
            vec.push(Some(chunk));
            vec[index].as_mut().unwrap()
        } else {
            assert!(vec[index].is_none());
            vec[index] = Some(chunk);
            vec[index].as_mut().unwrap()
        }
    }
}

pub struct World {
    chunks: HashMap<[i32; 2], ChunkColumn>,
    inserter: Mutex<(Generator, Vec<QueuedChunk>)>,
    blocks: Arc<BlockRegistry>,
}

impl World {
    pub fn gen_area(&self, pos: &BlockPos, range: i32) {
        let base = Self::chunk_at(pos);
        for x in (base[0] - range)..(base[0] + range + 1) {
            for y in (base[1] - range)..(base[1] + range + 1) {
                for z in (base[2] - range)..(base[2] + range + 1) {
                    self.create_chunk(&ChunkPos([x, y, z]));
                }
            }
        }
    }
    pub fn flush_chunk(&mut self) {
        let mut sources_to_trigger = UpdateQueue::new();
        let insert_pos;
        {
            let (_, ref mut buffer) = *self.inserter.get_mut().unwrap();
            if let Some(chunk) = buffer.pop() {
                insert_pos = chunk.pos.clone();
                let entry = self.chunks.entry([chunk.pos[0], chunk.pos[2]]);
                let column = entry.or_insert_with(|| ChunkColumn::new());
                column.insert(chunk.pos[1], Chunk {
                    natural_light: LightState::init_dark_chunk(),
                    data: chunk.data,
                    artificial_light: LightState::init_dark_chunk(),
                    update_render: AtomicBool::new(false)
                });
                for source in chunk.light_sources.iter() {
                    sources_to_trigger.push(source.1, source.0.clone(), None);
                }
            } else {
                return;
            }
        }
        let cs = CHUNK_SIZE as i32;
        let mut sky_light = UpdateQueue::new();
        if !self.chunk_loaded(&insert_pos.facing(Direction::PosY)) {
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let abs_pos = BlockPos([
                        cs * insert_pos[0] + x as i32,
                        cs * insert_pos[1] + cs - 1,
                        cs * insert_pos[2] + z as i32,
                    ]);
                    sky_light.push(MAX_NATURAL_LIGHT, abs_pos, Some(Direction::NegY));
                }
            }
        }
        for face in ALL_DIRECTIONS.iter() {
            let facing = insert_pos.facing(*face);
            if let Some(chunk) = self.borrow_chunk(&facing) {
                self.trigger_chunk_face_brightness(&facing, face.invert(), &mut sources_to_trigger, &mut sky_light);
                chunk.update_render.store(true, Ordering::Release);
            }
        }
        increase_light(&mut self.artificial_lightmap(insert_pos.clone()), sources_to_trigger);
        increase_light(&mut self.natural_lightmap(insert_pos.clone()), sky_light);

        //block natural light in chunk below
        if self.chunk_loaded(&insert_pos.facing(Direction::NegY)) {
            let mut relight = RelightData::new();
            let mut lm = self.natural_lightmap(insert_pos.clone());
            let inserted_cache = ChunkCache::new(insert_pos.clone(), &self.chunks).unwrap();
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let abs_pos = BlockPos([
                        insert_pos[0] * cs + x as i32,
                        insert_pos[1] * cs - 1,
                        insert_pos[2] * cs + z as i32,
                    ]);
                    if inserted_cache.chunk.natural_light[chunk_index(&[x, 0, z])].level() != MAX_NATURAL_LIGHT {
                        remove_light_rec(&mut lm, abs_pos, Direction::NegY, &mut relight);
                    }
                }
            }
            increase_light(&mut self.natural_lightmap(insert_pos.facing(Direction::NegY)), relight.build_queue(&mut lm));
        }
    }
    pub fn set_block(&self, pos: &BlockPos, block: BlockId) -> Result<(), ChunkAccessError> {
        let chunk_pos = Self::chunk_at(pos);
        if let Some(chunk) = self.borrow_chunk(&chunk_pos) {
            let block_pos = chunk_index_global(pos);
            let before;
            {
                before = chunk.data[block_pos].load();
                chunk.data[block_pos].store(block);
            }
            match (*self.blocks.light_type(before), *self.blocks.light_type(block)) {
                (LightType::Transparent, LightType::Transparent)
                | (LightType::Opaque, LightType::Opaque) => {},
                (LightType::Source(s1), LightType::Source(s2)) => {
                    if s2 > s1 {
                        increase_light(
                            &mut self.artificial_lightmap(chunk_pos),
                            UpdateQueue::single(s2, pos.clone(), None));
                    } else if s2 < s1 {
                        remove_and_relight(&mut self.artificial_lightmap(chunk_pos), &[pos.clone()]);
                    }
                },
                (LightType::Source(_), LightType::Transparent) => {
                    remove_and_relight(&mut self.artificial_lightmap(chunk_pos), &[pos.clone()]);
                },
                (LightType::Transparent, LightType::Source(s)) => {
                    increase_light(
                        &mut self.artificial_lightmap(chunk_pos),
                        UpdateQueue::single(s, pos.clone(), None));
                },
                (LightType::Opaque, _) => {
                    relight(&mut self.artificial_lightmap(chunk_pos.clone()), pos);
                    relight(&mut self.natural_lightmap(chunk_pos), pos);
                }
                (_, LightType::Opaque) => {
                    remove_and_relight(&mut self.artificial_lightmap(chunk_pos.clone()), &[pos.clone()]);
                    remove_and_relight(&mut self.natural_lightmap(chunk_pos), &[pos.clone()]);
                }
            }
            chunk.update_render.store(true, Ordering::Release);
            if self.blocks.is_opaque_draw(before) ^ self.blocks.is_opaque_draw(block) {
                self.update_adjacent_chunks(pos);
            }
            Ok(())
        } else {
            Err(ChunkAccessError::NoChunk)
        }
    }
    pub fn block_ray_trace(&self, start: [f32; 3], direction: [f32; 3], range: f32) -> Option<BlockIntersection> {
        for intersect in Ray::new(start, direction).blocks() {
            let sq_dist: f32 = intersect.block.0.iter()
                .map(|x| *x as f32 + 0.5)
                .zip(start.iter()).map(|x| x.1 - x.0)
                .map(|x| x * x).sum();
            if sq_dist > range * range {
                return None;
            }
            if let Some(id) = self.get_block(&intersect.block) {
                if self.blocks.is_opaque_draw(id) {
                    return Some(intersect)
                }
            }
        }
        unreachable!()// ray block iterator is infinite
    }
    pub fn get_block(&self, pos: &BlockPos) -> Option<BlockId> {
        self.borrow_chunk(&Self::chunk_at(pos)).map(|c| c.data[chunk_index_global(pos)].load())
    }
    pub fn natural_light(&self, pos: &BlockPos) -> Option<(u8, Option<Direction>)> {
        if let Some(chunk) = self.borrow_chunk(&Self::chunk_at(pos)) {
            let light = &chunk.natural_light[chunk_index_global(pos)];
            Some((light.level(), light.direction()))
        } else {
            None
        }
    }
    pub fn artificial_light(&self, pos: &BlockPos) -> Option<(u8, Option<Direction>)> {
        if let Some(chunk) = self.borrow_chunk(&Self::chunk_at(pos)) {
            let light = &chunk.artificial_light[chunk_index_global(pos)];
            Some((light.level(), light.direction()))
        } else {
            None
        }
    }
    fn update_adjacent_chunks(&self, block_pos: &BlockPos) {
        let cs = CHUNK_SIZE as i32;
        let chunk_pos = Self::chunk_at(block_pos);
        let (x, y, z) = (chunk_pos[0], chunk_pos[1], chunk_pos[2]);
        if block_pos[0].mod_floor(&cs) == 0 { self.update_render(&ChunkPos([x - 1, y, z])) }
        if block_pos[1].mod_floor(&cs) == 0 { self.update_render(&ChunkPos([x, y - 1, z])) }
        if block_pos[2].mod_floor(&cs) == 0 { self.update_render(&ChunkPos([x, y, z - 1])) }
        if block_pos[0].mod_floor(&cs) == cs - 1 { self.update_render(&ChunkPos([x + 1, y, z])) }
        if block_pos[1].mod_floor(&cs) == cs - 1 { self.update_render(&ChunkPos([x, y + 1, z])) }
        if block_pos[2].mod_floor(&cs) == cs - 1 { self.update_render(&ChunkPos([x, y, z + 1])) }
    }
    fn update_render(&self, pos: &ChunkPos) {
        if let Some(chunk) = self.borrow_chunk(pos) {
            chunk.update_render.store(true, Ordering::Release)
        }
    }
    fn borrow_chunk(&self, p: &ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&[p[0], p[2]]).and_then(|col| col.get(p[1]))
    }
    pub fn lock_chunk(&self, pos: &ChunkPos) -> Option<ChunkReader> {
        self.borrow_chunk(pos).map(|x| ChunkReader::new(x))
    }
    pub fn new(blocks: Arc<BlockRegistry>, generator: Generator) -> Self {
        World {
            chunks: HashMap::new(),
            blocks: blocks,
            inserter: Mutex::new((generator, Vec::new())),
        }
    }
    pub fn blocks(&self) -> &BlockRegistry {
        &*self.blocks
    }
    pub fn reset_chunk_updated(&self, pos: &ChunkPos) -> bool {
        self.borrow_chunk(pos).map(|c| c.update_render.swap(false, Ordering::Acquire)).unwrap_or(false)
    }
    pub fn chunk_loaded(&self, pos: &ChunkPos) -> bool {
        self.borrow_chunk(pos).is_some()
    }
    pub fn chunk_at(pos: &BlockPos) -> ChunkPos {
        use num::Integer;
        ChunkPos([
            pos[0].div_floor(&(CHUNK_SIZE as i32)),
            pos[1].div_floor(&(CHUNK_SIZE as i32)),
            pos[2].div_floor(&(CHUNK_SIZE as i32)),
        ])
    }
    fn create_chunk(&self, pos: &ChunkPos) {
        if self.borrow_chunk(pos).is_none() {
            let (ref mut generator, ref mut buffer) = *self.inserter.lock().unwrap();
            if !buffer.iter().any(|&ref chunk| chunk.pos == *pos) {
                let data = generator.gen_chunk(pos);
                let sources = (0..(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE)).filter_map(|i| {
                    match *self.blocks.light_type(data[i]) {
                        LightType::Source(l) => Some((BlockPos([
                            pos[0] * CHUNK_SIZE as i32 + (i / CHUNK_SIZE / CHUNK_SIZE) as i32,
                            pos[1] * CHUNK_SIZE as i32 + (i / CHUNK_SIZE % CHUNK_SIZE) as i32,
                            pos[2] * CHUNK_SIZE as i32 + (i % CHUNK_SIZE) as i32
                        ]), l)),
                        LightType::Opaque | LightType::Transparent => None,
                    }
                }).collect();
                let insert = QueuedChunk {
                    light_sources: sources,
                    pos: pos.clone(),
                    data: AtomicBlockId::init_chunk(&data),
                };
                buffer.push(insert);
            }
        }
    }

    fn trigger_chunk_face_brightness(&self,
                                     pos: &ChunkPos,
                                     face: Direction,
                                     artificial_updates: &mut UpdateQueue,
                                     natural_updates: &mut UpdateQueue) {
        let (positive, d1, d2, face_direction) = match face {
            Direction::PosX => (true, 1, 2, 0),
            Direction::NegX => (false, 1, 2, 0),
            Direction::PosY => (true, 0, 2, 1),
            Direction::NegY => (false, 0, 2, 1),
            Direction::PosZ => (true, 0, 1, 2),
            Direction::NegZ => (false, 0, 1, 2),
        };

        let mut brightness = [[(0, 0); CHUNK_SIZE]; CHUNK_SIZE];
        let chunk = self.borrow_chunk(pos).unwrap();
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut block_pos = [0, 0, 0];
                block_pos[d1] = i;
                block_pos[d2] = j;
                block_pos[face_direction] = if positive { CHUNK_SIZE - 1 } else { 0 };
                brightness[i][j].0 = chunk.artificial_light[chunk_index(&block_pos)].level();
                brightness[i][j].1 = chunk.natural_light[chunk_index(&block_pos)].level();
            }
        }
        let chunk_size = CHUNK_SIZE as i32;
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut block_pos = BlockPos([pos[0] * chunk_size, pos[1] * chunk_size, pos[2] * chunk_size]);
                block_pos.0[d1] += i as i32;
                block_pos.0[d2] += j as i32;
                block_pos.0[face_direction] += if positive { CHUNK_SIZE as i32 } else { -1 };

                if brightness[i][j].0 > 1 {
                    artificial_updates.push(brightness[i][j].0 - 1, block_pos.clone(), Some(face));
                }
                if brightness[i][j].1 > 1 {
                    if face == Direction::NegY && brightness[i][j].1 == MAX_NATURAL_LIGHT {
                        natural_updates.push(MAX_NATURAL_LIGHT, block_pos, Some(face));
                    } else {
                        natural_updates.push(brightness[i][j].1 - 1, block_pos, Some(face));
                    }
                }
            }
        }
    }
    fn artificial_lightmap(&self, p: ChunkPos) -> ArtificialLightMap {
        ArtificialLightMap {
            world: &self,
            cache: ChunkCache::new(p, &self.chunks).unwrap(),
        }
    }
    fn natural_lightmap(&self, p: ChunkPos) -> NaturalLightMap {
        NaturalLightMap {
            world: &self,
            cache: ChunkCache::new(p, &self.chunks).unwrap(),
        }
    }
}

struct ChunkCache<'a> {
    pos: ChunkPos,
    pub chunk: &'a Chunk,
}

impl<'a> ChunkCache<'a> {
    fn new<'b: 'a>(pos: ChunkPos, chunks: &'b HashMap<[i32; 2], ChunkColumn>) -> Result<Self, ()> {
        if let Some(cref) = chunks.get(&[pos[0], pos[2]]).and_then(|col| col.get(pos[1])) {
            Ok(ChunkCache {
                pos: pos,
                chunk: cref
            })
        } else {
            Err(())
        }
    }
    fn load<'b: 'a>(&mut self, pos: ChunkPos, chunks: &'b HashMap<[i32; 2], ChunkColumn>) -> Result<(), ()> {
        if pos == self.pos {
            Ok(())
        } else {
            *self = Self::new(pos, chunks)?;
            Ok(())
        }
    }
}

pub struct ArtificialLightMap<'a> {
    world: &'a World,
    cache: ChunkCache<'a>,
}

impl<'a> LightMap for ArtificialLightMap<'a> {
    fn is_opaque(&mut self, pos: &BlockPos) -> bool {
        if self.cache.load(World::chunk_at(pos), &self.world.chunks).is_err() {
            true
        } else {
            self.world.blocks.light_type(self.cache.chunk.data[chunk_index_global(pos)].load()).is_opaque()
        }
    }

    fn get_light(&mut self, pos: &BlockPos) -> Light {
        if self.cache.load(World::chunk_at(pos), &self.world.chunks).is_err() {
            (0, None)
        } else {
            let atomic_light = &self.cache.chunk.artificial_light[chunk_index_global(pos)];
            (atomic_light.level(), atomic_light.direction())
        }
    }

    fn set_light(&mut self, pos: &BlockPos, light: Light) {
        self.cache.chunk.artificial_light[chunk_index_global(pos)].set(light.0, light.1);
        self.cache.chunk.update_render.store(true, Ordering::Release);
        self.world.update_adjacent_chunks(pos);
    }
    fn compute_light_to(&mut self, _: Direction, level: u8) -> u8 {
        level - 1
    }
    fn internal_light(&mut self, pos: &BlockPos) -> u8 {
        if self.cache.load(World::chunk_at(pos), &self.world.chunks).is_err() {
            0
        } else {
            match *self.world.blocks.light_type(self.cache.chunk.data[chunk_index_global(pos)].load()) {
                LightType::Source(s) => s,
                LightType::Opaque | LightType::Transparent => 0
            }
        }
    }
}

pub struct NaturalLightMap<'a> {
    world: &'a World,
    cache: ChunkCache<'a>,
}

impl<'a> LightMap for NaturalLightMap<'a> {
    fn is_opaque(&mut self, pos: &BlockPos) -> bool {
        if self.cache.load(World::chunk_at(pos), &self.world.chunks).is_err() {
            true
        } else {
            self.world.blocks.light_type(self.cache.chunk.data[chunk_index_global(pos)].load()).is_opaque()
        }
    }

    fn get_light(&mut self, pos: &BlockPos) -> Light {
        if self.cache.load(World::chunk_at(pos), &self.world.chunks).is_err() {
            (0, None)
        } else {
            let atomic_light = &self.cache.chunk.natural_light[chunk_index_global(pos)];
            (atomic_light.level(), atomic_light.direction())
        }
    }

    fn set_light(&mut self, pos: &BlockPos, light: Light) {
        self.cache.chunk.natural_light[chunk_index_global(pos)].set(light.0, light.1);
        self.cache.chunk.update_render.store(true, Ordering::Release);
        self.world.update_adjacent_chunks(pos);
    }

    fn compute_light_to(&mut self, d: Direction, level: u8) -> u8 {
        if level == MAX_NATURAL_LIGHT && d == Direction::NegY {
            MAX_NATURAL_LIGHT
        } else {
            level - 1
        }
    }

    fn internal_light(&mut self, _: &BlockPos) -> u8 {
        0
    }
}