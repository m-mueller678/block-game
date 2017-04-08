mod generator;
mod atomic_light;
mod chunk;
mod lighting;

pub use self::chunk::{ChunkReader, chunk_index, chunk_index_global, CHUNK_SIZE};
pub use self::generator::Generator;

use block::{AtomicBlockId, BlockId, BlockRegistry, LightType};
use self::lighting::*;
use std::collections::hash_map::HashMap;
use geometry::{Direction, ALL_DIRECTIONS};
use geometry::ray::Ray;
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use num::Integer;
use self::atomic_light::{LightState};
use self::chunk::{Chunk, init_vertical_clear, update_vertical_clear};
use std;

pub struct World {
    chunks: HashMap<[i32; 2], ChunkColumn>,
    inserter: Mutex<(Generator, Vec<QueuedChunk>)>,
    blocks: Arc<BlockRegistry>,
}


#[derive(Debug)]
pub enum ChunkAccessError {
    NoChunk
}

struct QueuedChunk {
    light_sources: Vec<([i32; 3], u8)>,
    pos: [i32; 3],
    data: [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    vertical_clear: [AtomicBool; CHUNK_SIZE * CHUNK_SIZE],
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

impl World {
    pub fn gen_area(&self, pos: &[i32; 3], range: i32) {
        let base = Self::chunk_at(pos);
        for x in (base[0] - range)..(base[0] + range + 1) {
            for y in (base[1] - range)..(base[1] + range + 1) {
                for z in (base[2] - range)..(base[2] + range + 1) {
                    self.create_chunk(&[x, y, z]);
                }
            }
        }
    }
    pub fn flush_chunks(&mut self, preferred_flush_count: usize, max_rest: usize) {
        let mut source_buffer = Vec::new();
        let mut face_buffer = Vec::new();
        {
            let (_, ref mut buffer) = *self.inserter.get_mut().unwrap();
            let flush_count = if buffer.len() <= max_rest + preferred_flush_count {
                std::cmp::min(buffer.len(), preferred_flush_count)
            } else {
                buffer.len() - max_rest
            };
            for chunk in buffer.drain(0..flush_count) {
                let entry = self.chunks.entry([chunk.pos[0], chunk.pos[2]]);
                let column = entry.or_insert_with(|| ChunkColumn::new());
                column.insert(chunk.pos[1], Chunk {
                    vertical_clear: chunk.vertical_clear,
                    data: chunk.data,
                    light: LightState::init_dark_chunk(),
                    update_render: AtomicBool::new(false)
                });
                for d in ALL_DIRECTIONS.iter() {
                    let other_pos = d.apply_to_pos(chunk.pos);
                    face_buffer.push((other_pos, d.invert()));
                }
                for source in chunk.light_sources {
                    source_buffer.push(source);
                }
            }
        }
        for &(pos, level) in source_buffer.iter() {
            self.source_increase_brightness(&pos, level);
        }
        for &(ref chunk_pos, face) in face_buffer.iter() {
            if let Some(chunk) = self.borrow_chunk(chunk_pos) {
                self.trigger_chunk_face_brightness(&chunk_pos, face);
                chunk.update_render.store(true, Ordering::Release);
            }
        }
    }
    pub fn set_block(&self, pos: &[i32; 3], block: BlockId) -> Result<(), ChunkAccessError> {
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
                        self.source_increase_brightness(pos, s2);
                    } else if s2 < s1 {
                        decrease_light(&mut self.artificial_lightmap(Self::chunk_at(pos)), pos, (s2, Direction::PosX));
                    }
                },
                (LightType::Source(_), LightType::Transparent) => {
                    self.remove_brightness(pos);
                    self.transparent_increase_brightness(pos);
                },
                (_, LightType::Source(s)) => {
                    self.source_increase_brightness(pos, s);
                }
                (LightType::Opaque, LightType::Transparent) => {
                    self.transparent_increase_brightness(pos);
                },
                (_, LightType::Opaque) => {
                    self.remove_brightness(pos);
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
    pub fn block_ray_trace(&self, start: [f32; 3], direction: [f32; 3], range: f32) -> Option<[i32; 3]> {
        for block_pos in Ray::new(start, direction).blocks() {
            let sq_dist: f32 = block_pos.iter()
                .map(|x| *x as f32 + 0.5)
                .zip(start.iter()).map(|x| x.1 - x.0)
                .map(|x| x * x).sum();
            if sq_dist > range * range {
                return None;
            }
            if let Some(id) = self.get_block(&block_pos) {
                if self.blocks.is_opaque_draw(id) {
                    return Some(block_pos)
                }
            }
        }
        unreachable!()// ray block iterator is infinite
    }
    pub fn get_block(&self, pos: &[i32; 3]) -> Option<BlockId> {
        self.borrow_chunk(&Self::chunk_at(pos)).map(|c| c.data[chunk_index_global(pos)].load())
    }
    fn update_adjacent_chunks(&self, block_pos: &[i32; 3]) {
        let cs = CHUNK_SIZE as i32;
        let (x, y, z) = (block_pos[0].div_floor(&cs), block_pos[1].div_floor(&cs), block_pos[2].div_floor(&cs));
        if block_pos[0].mod_floor(&cs) == 0 { self.update_render(&[x - 1, y, z]) }
        if block_pos[1].mod_floor(&cs) == 0 { self.update_render(&[x, y - 1, z]) }
        if block_pos[2].mod_floor(&cs) == 0 { self.update_render(&[x, y, z - 1]) }
        if block_pos[0].mod_floor(&cs) == cs - 1 { self.update_render(&[x + 1, y, z]) }
        if block_pos[1].mod_floor(&cs) == cs - 1 { self.update_render(&[x, y + 1, z]) }
        if block_pos[2].mod_floor(&cs) == cs - 1 { self.update_render(&[x, y, z + 1]) }
    }
    fn update_render(&self, pos: &[i32; 3]) {
        if let Some(chunk) = self.borrow_chunk(pos) {
            chunk.update_render.store(true, Ordering::Release)
        }
    }
    fn borrow_chunk(&self, p: &[i32; 3]) -> Option<&Chunk> {
        self.chunks.get(&[p[0], p[2]]).and_then(|col| col.get(p[1]))
    }
    pub fn lock_chunk(&self, pos: &[i32; 3]) -> Option<ChunkReader> {
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
    pub fn reset_chunk_updated(&self, pos: &[i32; 3]) -> bool {
        self.borrow_chunk(pos).map(|c| c.update_render.swap(false, Ordering::Acquire)).unwrap_or(false)
    }
    pub fn chunk_loaded(&self, pos: &[i32; 3]) -> bool {
        self.borrow_chunk(pos).is_some()
    }
    fn chunk_at(pos: &[i32; 3]) -> [i32; 3] {
        use num::Integer;
        [
            pos[0].div_floor(&(CHUNK_SIZE as i32)),
            pos[1].div_floor(&(CHUNK_SIZE as i32)),
            pos[2].div_floor(&(CHUNK_SIZE as i32)),
        ]
    }
    fn create_chunk(&self, pos: &[i32; 3]) {
        if self.borrow_chunk(pos).is_none() {
            let (ref mut generator, ref mut buffer) = *self.inserter.lock().unwrap();
            if !buffer.iter().any(|&ref chunk| chunk.pos == *pos) {
                let data = generator.gen_chunk(pos);
                let sources = (0..(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE)).filter_map(|i| {
                    match *self.blocks.light_type(data[i]) {
                        LightType::Source(l) => Some(([
                                                          (i / CHUNK_SIZE / CHUNK_SIZE) as i32,
                                                          (i / CHUNK_SIZE % CHUNK_SIZE) as i32,
                                                          (i % CHUNK_SIZE) as i32
                                                      ], l)),
                        LightType::Opaque | LightType::Transparent => None,
                    }
                }).collect();
                let insert = QueuedChunk {
                    vertical_clear: init_vertical_clear(),
                    light_sources: sources,
                    pos: *pos,
                    data: AtomicBlockId::init_chunk(&data),
                };
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        update_vertical_clear(&insert.vertical_clear, &insert.data, [x, z], &*self.blocks);
                    }
                }
                buffer.push(insert);
            }
        }
    }

    fn artificial_lightmap(&self, p: [i32; 3]) -> ArtificialLightMap {
        ArtificialLightMap {
            world: &self,
            cache: ChunkCache::new(p, &self.chunks).unwrap(),
        }
    }
    fn transparent_increase_brightness(&self, p: &[i32; 3]) {
        use std::cmp::Ord;
        let mut lm = self.artificial_lightmap(Self::chunk_at(p));
        let max_light = ALL_DIRECTIONS.iter()
            .map(|direction| (self.get_brightness(&direction.apply_to_pos(*p), &mut lm.cache).unwrap_or(0), *direction))
            .max_by(|light1, light2| light1.0.cmp(&light2.0)).unwrap();
        increase_light(&mut lm, vec![(*p, max_light.1)], max_light.0);
    }
    fn source_increase_brightness(&self, p: &[i32; 3], level: u8) {
        increase_light(&mut self.artificial_lightmap(Self::chunk_at(p)), vec![(*p, Direction::PosX)], level);
    }
    fn remove_brightness(&self, p: &[i32; 3]) {
        let mut lm = self.artificial_lightmap(Self::chunk_at(p));
        let old_light = lm.get_light(p);
        remove_light(&mut lm, vec![(*p, old_light.1)], old_light.0);
    }
    fn trigger_chunk_face_brightness(&self, pos: &[i32; 3], face: Direction) {
        let (positive, d1, d2, face_direction) = match face {
            Direction::PosX => (true, 1, 2, 0),
            Direction::NegX => (false, 1, 2, 0),
            Direction::PosY => (true, 0, 2, 1),
            Direction::NegY => (false, 0, 2, 1),
            Direction::PosZ => (true, 0, 1, 2),
            Direction::NegZ => (false, 0, 1, 2),
        };

        let mut brightness = [[0; CHUNK_SIZE]; CHUNK_SIZE];
        let mut lm = self.artificial_lightmap(face.apply_to_pos(*pos));
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut block_pos = [0, 0, 0];
                block_pos[d1] = i;
                block_pos[d2] = j;
                block_pos[face_direction] = if positive { CHUNK_SIZE - 1 } else { 0 };
                brightness[i][j] = lm.cache.chunk.light[chunk_index(&block_pos)].level();
            }
        }
        let chunk_size = CHUNK_SIZE as i32;
        let mut updates: [Vec<([i32; 3], Direction)>; 15] = [Vec::new(), Vec::new(), Vec::new(), Vec::new(),
            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
            Vec::new(), Vec::new(), Vec::new()];
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                if brightness[i][j] <= 1 {
                    continue;
                }
                let mut pos = [pos[0] * chunk_size, pos[1] * chunk_size, pos[2] * chunk_size];
                pos[d1] += i as i32;
                pos[d2] += j as i32;
                pos[face_direction] += if positive { 0 } else { CHUNK_SIZE as i32 - 1 };
                assert!(brightness[i][j] < 16);
                assert!(brightness[i][j] > 1);
                updates[brightness[i][j] as usize - 1].push((pos, face));
            }
        }
        for (i, pos) in updates.iter_mut().map(|v| {
            let mut move_to = vec![];
            move_to.append(v);
            move_to
        }).enumerate() {
            increase_light(&mut lm, pos, i as u8 + 1);
        }
    }
    fn get_brightness<'a: 'b, 'b>(&'a self, pos: &[i32; 3], cache: &mut ChunkCache<'b>) -> Option<u8> {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = chunk_index_global(pos);
        if cache.load(chunk_pos, &self.chunks).is_ok() {
            Some(cache.chunk.light[block_position].level())
        } else {
            None
        }
    }
}

struct ChunkCache<'a> {
    pos: [i32; 3],
    chunk: &'a Chunk,
}

impl<'a> ChunkCache<'a> {
    fn new<'b: 'a>(pos: [i32; 3], chunks: &'b HashMap<[i32; 2], ChunkColumn>) -> Result<Self, ()> {
        if let Some(cref) = chunks.get(&[pos[0], pos[2]]).and_then(|col| col.get(pos[1])) {
            Ok(ChunkCache {
                pos: pos,
                chunk: cref
            })
        } else {
            Err(())
        }
    }
    fn load<'b: 'a>(&mut self, pos: [i32; 3], chunks: &'b HashMap<[i32; 2], ChunkColumn>) -> Result<(), ()> {
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
    fn is_opaque(&mut self, pos: &[i32; 3]) -> bool {
        if self.cache.load(World::chunk_at(pos), &self.world.chunks).is_err() {
            true
        } else {
            match *self.world.blocks.light_type(self.cache.chunk.data[chunk_index_global(pos)].load()) {
                LightType::Opaque => true,
                LightType::Source(_) => false,
                LightType::Transparent => false,
            }
        }
    }

    fn get_light(&mut self, pos: &[i32; 3]) -> Light {
        if self.cache.load(World::chunk_at(pos), &self.world.chunks).is_err() {
            (0, Direction::PosX)
        } else {
            let atomic_light = &self.cache.chunk.light[chunk_index_global(pos)];
            (atomic_light.level(), atomic_light.direction())
        }
    }

    fn set_light(&mut self, pos: &[i32; 3], light: Light) {
        self.cache.chunk.light[chunk_index_global(pos)].set(light.0, light.1);
        self.cache.chunk.update_render.store(true, Ordering::Release);
    }
}