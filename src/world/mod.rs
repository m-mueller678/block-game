mod generator;
mod atomic_light;
mod chunk;

pub use self::chunk::{ChunkReader, chunk_index, chunk_index_global, CHUNK_SIZE};
pub use self::generator::Generator;

use block::{AtomicBlockId, BlockId, BlockRegistry, LightType};
use std::collections::hash_map::{HashMap};
use geometry::{Direction, ALL_DIRECTIONS};
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use num::Integer;
use self::atomic_light::{LightState};
use self::chunk::Chunk;

pub struct World {
    chunks: HashMap<[i32; 3], Chunk>,
    inserter: Mutex<(Generator, Vec<QueuedChunk>)>,
    blocks: Arc<BlockRegistry>,
}


#[derive(Debug)]
pub enum ChunkAccessError {
    NoChunk
}

struct QueuedChunk {
    light_sources: Vec<usize>,
    pos: [i32; 3],
    data: [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
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
    pub fn flush_chunks(&mut self) {
        let mut source_buffer = Vec::new();
        let mut face_buffer = Vec::new();
        {
            use std::mem::replace;
            let (_, ref mut buffer) = *self.inserter.get_mut().unwrap();
            let buffer = replace(buffer, Vec::new());
            for chunk in buffer.into_iter() {
                self.chunks.insert(chunk.pos, Chunk {
                    data: chunk.data,
                    light: LightState::init_dark_chunk(),
                    update_render: AtomicBool::new(false)
                });
                for d in ALL_DIRECTIONS.iter() {
                    let other_pos = d.apply_to_pos(chunk.pos);
                    face_buffer.push((other_pos, d.invert()));
                }
                for source in chunk.light_sources {
                    let abs_block_pos = [
                        chunk.pos[0] * CHUNK_SIZE as i32 + (source / CHUNK_SIZE / CHUNK_SIZE) as i32,
                        chunk.pos[1] * CHUNK_SIZE as i32 + (source / CHUNK_SIZE % CHUNK_SIZE) as i32,
                        chunk.pos[2] * CHUNK_SIZE as i32 + (source & CHUNK_SIZE) as i32,
                    ];
                    source_buffer.push(abs_block_pos);
                }
            }
        }
        for source in source_buffer.iter() {
            self.brightness_increased(&source);
        }
        for &(ref chunk_pos, face) in face_buffer.iter() {
            if let Some(chunk) = self.chunks.get(chunk_pos) {
                self.trigger_chunk_face_brightness(&chunk_pos, face);
                chunk.update_render.store(true, Ordering::Release);
            }
        }
    }
    pub fn set_block(&self, pos: &[i32; 3], block: BlockId) -> Result<(), ChunkAccessError> {
        let chunk_pos = Self::chunk_at(pos);
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
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
                        self.brightness_increased(pos);
                    } else if s2 < s1 {
                        self.brightness_decreased(pos);
                    }
                },
                (LightType::Source(_), LightType::Transparent) => {
                    self.brightness_decreased(pos);
                },
                (_, LightType::Source(_))
                | (LightType::Opaque, LightType::Transparent) => {
                    self.brightness_increased(pos);
                },
                (_, LightType::Opaque) => {
                    self.brightness_blocked(pos);
                }
            }
            chunk.update_render.store(true, Ordering::Release);
            if self.blocks.is_opaque(before) ^ self.blocks.is_opaque(block) {
                self.update_adjacent_chunks(pos);
            }
            Ok(())
        } else {
            Err(ChunkAccessError::NoChunk)
        }
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
        if let Some(chunk) = self.chunks.get(pos) {
            chunk.update_render.store(true, Ordering::Release)
        }
    }
    pub fn lock_chunk(&self, pos: &[i32; 3]) -> Option<ChunkReader> {
        self.chunks.get(pos).map(|x| ChunkReader::new(x))
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
        self.chunks.get(pos).map(|c| c.update_render.load(Ordering::Acquire)).unwrap_or(false)
    }
    pub fn chunk_loaded(&self, pos: &[i32; 3]) -> bool {
        self.chunks.contains_key(pos)
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
        if !self.chunks.contains_key(pos) {
            let (ref mut generator, ref mut buffer) = *self.inserter.lock().unwrap();
            if !buffer.iter().any(|&ref chunk| chunk.pos == *pos) {
                let data = generator.gen_chunk(pos);
                let sources = (0..(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE)).filter(|i| {
                    match *self.blocks.light_type(data[*i]) {
                        LightType::Source(_) => true,
                        LightType::Opaque | LightType::Transparent => false,
                    }
                }).collect();
                buffer.push(QueuedChunk {
                    light_sources: sources,
                    pos: *pos,
                    data: AtomicBlockId::init_chunk(&data),
                });
            }
        }
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
        let mut chunk = ChunkCache::new(face.apply_to_pos(*pos), &self.chunks).unwrap();
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut block_pos = [0, 0, 0];
                block_pos[d1] = i;
                block_pos[d2] = j;
                block_pos[face_direction] = if positive { CHUNK_SIZE - 1 } else { 0 };
                brightness[i][j] = chunk.chunk.light[chunk_index(&block_pos)].level();
            }
        }
        let chunk_size = CHUNK_SIZE as i32;
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                if brightness[i][j] <= 1 {
                    continue;
                }
                let mut pos = [pos[0] * chunk_size, pos[1] * chunk_size, pos[2] * chunk_size];
                pos[d1] += i as i32;
                pos[d2] += j as i32;
                pos[face_direction] += if positive { 0 } else { CHUNK_SIZE as i32 - 1 };
                self.brightness_increased_rec(&pos, &mut chunk, (brightness[i][j] - 1, face))
            }
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
    fn brightness_increased(&self, pos: &[i32; 3]) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = chunk_index_global(pos);
        let mut cache = ChunkCache::new(chunk_pos, &self.chunks).unwrap();
        let current_brightness = cache.chunk.light[block_position].level();
        let own_light_level = match *self.blocks.light_type(cache.chunk.data[block_position].load()) {
            LightType::Source(p) => p,
            LightType::Transparent | LightType::Opaque => 0,
        };
        let mut light = (own_light_level, Direction::PosX);
        for d in ALL_DIRECTIONS.iter() {
            let adjacent_light = self.get_brightness(&d.apply_to_pos(*pos), &mut cache).unwrap_or(0);
            if adjacent_light > light.0 + 1 {
                light = (adjacent_light - 1, *d)
            }
        }
        assert!(light.0 >= current_brightness);
        if light.0 > current_brightness {
            cache.chunk.light[block_position].set(light.0, light.1);
            for d in ALL_DIRECTIONS.iter() {
                self.brightness_increased_rec(&d.apply_to_pos(*pos), &mut cache, (light.0 - 1, *d))
            }
        }
    }
    fn brightness_decreased(&self, pos: &[i32; 3]) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = chunk_index_global(pos);
        let mut cache = ChunkCache::new(chunk_pos, &self.chunks).unwrap();
        let direction = cache.chunk.light[block_position].direction();
        self.light_source_removed_rec(pos, &mut cache, direction);
    }
    fn brightness_blocked(&self, pos: &[i32; 3]) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = chunk_index_global(pos);
        let mut cache = ChunkCache::new(chunk_pos, &self.chunks).unwrap();
        cache.chunk.light[block_position].set_level(0);
        for d in ALL_DIRECTIONS.iter() {
            self.light_source_removed_rec(&d.apply_to_pos(*pos), &mut cache, *d);
        }
    }
    fn brightness_increased_rec<'a: 'b, 'b>(&'a self, pos: &[i32; 3], cache: &mut ChunkCache<'b>, brightness: (u8, Direction)) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = chunk_index_global(pos);
        if cache.load(chunk_pos, &self.chunks).is_err() {
            return;
        }
        match *self.blocks.light_type(cache.chunk.data[block_position].load()) {
            LightType::Transparent | LightType::Source(_) => {
                if cache.chunk.light[block_position].level() < brightness.0 {
                    cache.chunk.light[block_position].set(brightness.0, brightness.1);
                    cache.chunk.update_render.store(true, Ordering::Release);
                    if brightness.0 > 1 {
                        for d in ALL_DIRECTIONS.iter() {
                            self.brightness_increased_rec(&d.apply_to_pos(*pos), cache, (brightness.0 - 1, *d));
                        }
                    }
                }
            }
            LightType::Opaque => {}
        }
    }
    fn light_source_removed_rec<'a: 'b, 'b>(&'a self, pos: &[i32; 3], cache: &mut ChunkCache<'b>, direction: Direction) -> u8 {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = chunk_index_global(pos);
        if cache.load(chunk_pos, &self.chunks).is_err() {
            return 0;
        }
        if direction == cache.chunk.light[block_position].direction() {
            let mut own_brightness = match *self.blocks.light_type(cache.chunk.data[block_position].load()) {
                LightType::Transparent => 0,
                LightType::Source(strength) => strength,
                LightType::Opaque => { return 0; },
            };
            if own_brightness == cache.chunk.light[block_position].level() { return own_brightness; }
            cache.chunk.light[block_position].set_level(own_brightness);
            cache.chunk.update_render.store(true, Ordering::Release);
            let mut light_from = None;
            for d in ALL_DIRECTIONS.iter() {
                let adjacent_brightness = self.light_source_removed_rec(&d.apply_to_pos(*pos), cache, *d);
                if adjacent_brightness > own_brightness + 1 {
                    own_brightness = adjacent_brightness - 1;
                    light_from = Some(*d);
                }
            }
            if let Some(light_direction) = light_from.map(|d| d.invert()) {
                cache.load(*pos, &self.chunks).expect("restore ChunkCache");
                cache.chunk.light[block_position].set(own_brightness, light_direction);
                for d in ALL_DIRECTIONS.iter() {
                    self.brightness_increased_rec(&d.apply_to_pos(*pos), cache, (own_brightness - 1, *d));
                }
            }
            own_brightness
        } else {
            cache.chunk.light[block_position].level()
        }
    }
}

struct ChunkCache<'a> {
    pos: [i32; 3],
    chunk: &'a Chunk,
}

impl<'a> ChunkCache<'a> {
    fn new<'b: 'a>(pos: [i32; 3], chunks: &'b HashMap<[i32; 3], Chunk>) -> Result<Self, ()> {
        if let Some(cref) = chunks.get(&pos) {
            Ok(ChunkCache {
                pos: pos,
                chunk: cref
            })
        } else {
            Err(())
        }
    }
    fn load<'b: 'a>(&mut self, pos: [i32; 3], chunks: &'b HashMap<[i32; 3], Chunk>) -> Result<(), ()> {
        if pos == self.pos {
            Ok(())
        } else {
            if let Some(cref) = chunks.get(&pos) {
                self.pos = pos;
                self.chunk = cref;
                Ok(())
            } else {
                Err(())
            }
        }
    }
}
