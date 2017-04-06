pub use self::generator::Generator;
use block::{BlockId, BlockRegistry, LightType};
use std::collections::hash_map::{HashMap};
use chunk::{Chunk, CHUNK_SIZE, RenderChunk, ChunkUniforms};
use geometry::direction::{Direction, ALL_DIRECTIONS};
use chunk::block_graphics_supplier::BlockGraphicsSupplier;
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use std::ops::Deref;
use glium;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;
use num::Integer;

mod generator;

pub struct World {
    chunks: HashMap<[i32; 3], ChunkData>,
    inserter: Mutex<(Generator, Vec<QueuedChunk>)>,
    blocks: Arc<BlockRegistry>,
}

#[derive(Debug)]
pub enum ChunkAccessError {
    NoChunk
}

struct ChunkData {
    chunk: Mutex<Chunk>,
    update_render: AtomicBool,
}

struct ChunkCache<'a> {
    pos: [i32; 3],
    guard: MutexGuard<'a, Chunk>,
    update_render: &'a AtomicBool,
}

impl<'a> ChunkCache<'a> {
    fn new<'b: 'a>(pos: [i32; 3], chunks: &'b HashMap<[i32; 3], ChunkData>) -> Result<Self, ()> {
        if let Some(cd) = chunks.get(&pos) {
            Ok(ChunkCache {
                pos: pos,
                guard: cd.chunk.lock().unwrap(),
                update_render: &cd.update_render,
            })
        } else {
            Err(())
        }
    }
    fn load<'b: 'a>(&mut self, pos: [i32; 3], chunks: &'b HashMap<[i32; 3], ChunkData>) -> Result<(), ()> {
        if pos == self.pos {
            Ok(())
        } else {
            if let Some(cd) = chunks.get(&pos) {
                self.guard = cd.chunk.lock().unwrap();
                self.pos = pos;
                self.update_render = &cd.update_render;
                Ok(())
            } else {
                Err(())
            }
        }
    }
}

pub struct ChunkReader<'a> {
    lock: MutexGuard<'a, Chunk>
}

impl<'a> Deref for ChunkReader<'a> {
    type Target = Chunk;
    fn deref(&self) -> &Chunk {
        self.lock.deref()
    }
}

struct QueuedChunk {
    light_sources: Vec<usize>,
    pos: [i32; 3],
    data: [BlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
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
                self.chunks.insert(chunk.pos, ChunkData {
                    chunk: Mutex::new(Chunk {
                        data: chunk.data,
                        light: [(0, Direction::PosX); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
                    }),
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
            let block_pos = Chunk::index(pos);
            let before;
            {
                let mut lock = chunk.chunk.lock().unwrap();
                before = lock.data[block_pos];
                lock.data[block_pos] = block;
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
        self.chunks.get(pos).map(|x| ChunkReader { lock: x.chunk.lock().unwrap() })
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
                    data: data,
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
                brightness[i][j] = chunk.guard.light[Chunk::u_index(&block_pos)].0;
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
    fn get_brightness<'a: 'b, 'b>(&'a self, pos: &[i32; 3], chunk: &mut ChunkCache<'b>) -> Option<u8> {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = Chunk::index(pos);
        if chunk.load(chunk_pos, &self.chunks).is_ok() {
            Some(chunk.guard.light[block_position].0)
        } else {
            None
        }
    }
    fn brightness_increased(&self, pos: &[i32; 3]) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = Chunk::index(pos);
        let mut chunk = ChunkCache::new(chunk_pos, &self.chunks).unwrap();
        let current_brightness = chunk.guard.light[block_position].0;
        let mut light = (match *self.blocks.light_type(chunk.guard.data[block_position]) {
            LightType::Source(p) => p,
            LightType::Transparent | LightType::Opaque => 0,
        }, Direction::PosX);
        for d in ALL_DIRECTIONS.iter() {
            let adjacent_light = self.get_brightness(&d.apply_to_pos(*pos), &mut chunk).unwrap_or(0);
            if adjacent_light > light.0 + 1 {
                light = (adjacent_light - 1, *d)
            }
        }
        assert!(light.0 >= current_brightness);
        if light.0 > current_brightness {
            chunk.guard.light[block_position] = light;
            for d in ALL_DIRECTIONS.iter() {
                self.brightness_increased_rec(&d.apply_to_pos(*pos), &mut chunk, (light.0 - 1, *d))
            }
        }
    }
    fn brightness_decreased(&self, pos: &[i32; 3]) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = Chunk::index(pos);
        let mut chunk = ChunkCache::new(chunk_pos, &self.chunks).unwrap();
        let direction = chunk.guard.light[block_position].1;
        self.light_source_removed_rec(pos, &mut chunk, direction);
    }
    fn brightness_blocked(&self, pos: &[i32; 3]) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = Chunk::index(pos);
        let mut chunk = ChunkCache::new(chunk_pos, &self.chunks).unwrap();
        chunk.guard.light[block_position].0 = 0;
        for d in ALL_DIRECTIONS.iter() {
            self.light_source_removed_rec(&d.apply_to_pos(*pos), &mut chunk, *d);
        }
    }
    fn brightness_increased_rec<'a: 'b, 'b>(&'a self, pos: &[i32; 3], chunk: &mut ChunkCache<'b>, brightness: (u8, Direction)) {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = Chunk::index(pos);
        if chunk.load(chunk_pos, &self.chunks).is_err() {
            return;
        }
        match *self.blocks.light_type(chunk.guard.data[block_position]) {
            LightType::Transparent | LightType::Source(_) => {
                if chunk.guard.light[block_position].0 < brightness.0 {
                    chunk.guard.light[block_position] = brightness;
                    chunk.update_render.store(true, Ordering::Release);
                    if brightness.0 > 1 {
                        for d in ALL_DIRECTIONS.iter() {
                            self.brightness_increased_rec(&d.apply_to_pos(*pos), chunk, (brightness.0 - 1, *d));
                        }
                    }
                }
            }
            LightType::Opaque => {}
        }
    }
    fn light_source_removed_rec<'a: 'b, 'b>(&'a self, pos: &[i32; 3], chunk: &mut ChunkCache<'b>, direction: Direction) -> u8 {
        let chunk_pos = Self::chunk_at(pos);
        let block_position = Chunk::index(pos);
        if chunk.load(chunk_pos, &self.chunks).is_err() {
            return 0;
        }
        if direction == chunk.guard.light[block_position].1 {
            let mut own_brightness = match *self.blocks.light_type(chunk.guard.data[block_position]) {
                LightType::Transparent => 0,
                LightType::Source(strength) => strength,
                LightType::Opaque => { return 0; },
            };
            if own_brightness == chunk.guard.light[block_position].0 { return own_brightness; }
            chunk.guard.light[block_position].0 = own_brightness;
            chunk.update_render.store(true, Ordering::Release);
            let mut light_from = None;
            for d in ALL_DIRECTIONS.iter() {
                let adjacent_brightness = self.light_source_removed_rec(&d.apply_to_pos(*pos), chunk, *d);
                if adjacent_brightness > own_brightness + 1 {
                    own_brightness = adjacent_brightness - 1;
                    light_from = Some(*d);
                }
            }
            if let Some(light_direction) = light_from.map(|d| d.invert()) {
                chunk.load(*pos, &self.chunks).expect("restore ChunkCache");
                chunk.guard.light[block_position] = (own_brightness, light_direction);
                for d in ALL_DIRECTIONS.iter() {
                    self.brightness_increased_rec(&d.apply_to_pos(*pos), chunk, (own_brightness - 1, *d));
                }
            }
            own_brightness
        } else {
            chunk.guard.light[block_position].0
        }
    }
}

pub struct WorldRender<'a, F: 'a + glium::backend::Facade> {
    render_dist: i32,
    render_chunks: Vec<([i32; 3], RenderChunk)>,
    facade: &'a F,
}

impl<'a, F: glium::backend::Facade> WorldRender<'a, F> {
    pub fn new(facade: &'a F) -> Self {
        WorldRender {
            render_dist: 4,
            render_chunks: Vec::new(),
            facade: facade,
        }
    }
    pub fn draw<S: glium::Surface>(&self, surface: &mut S,
                                   transform: [[f32; 4]; 4],
                                   sampler: Sampler<CompressedSrgbTexture2dArray>)
                                   -> Result<(), glium::DrawError> {
        use geometry::CORNER_OFFSET;
        use vecmath::*;
        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };
        let uniforms = ChunkUniforms {
            transform: transform,
            light: [0., -2., 1.],
            sampler: sampler,
        };
        let chunk_iter = self.render_chunks.iter().filter(|&&(pos, _)| {
            let corners: Vec<[f32; 3]> = CORNER_OFFSET.iter().
                map(|c| [c[0] + pos[0] as f32, c[1] + pos[1] as f32, c[2] + pos[2] as f32])
                .map(|c| vec3_scale(c, CHUNK_SIZE as f32))
                .map(|c| col_mat4_transform(transform, [c[0], c[1], c[2], 1.]))
                .map(|c| [c[0] / c[3], c[1] / c[3], c[2] / c[3]]).collect();
            corners.iter().any(|c| c[2] < 1.)
                & &corners.iter().any(|c| c[0] < 1.)
                & &corners.iter().any(|c| c[0] > -1.)
                & &corners.iter().any(|c| c[1] < 1.)
                & &corners.iter().any(|c| c[1] > -1.)
        });
        for chunk in chunk_iter {
            chunk.1.draw(surface, &uniforms, &params)?;
        }
        Ok(())
    }
    pub fn update(&mut self, pos: &[i32; 3], world: &World) {
        let chunk_size = CHUNK_SIZE as i32;
        let chunk_index = [pos[0] / chunk_size, pos[1] / chunk_size, pos[2] / chunk_size];
        let render_dist = self.render_dist;
        self.render_chunks.retain(|&(pos, _)| {
            (pos[0] - chunk_index[0]).abs() <= render_dist
                && (pos[1] - chunk_index[1]).abs() <= render_dist
                && (pos[2] - chunk_index[2]).abs() <= render_dist
        });
        let range = -self.render_dist..(self.render_dist + 1);
        for x in range.clone() {
            for y in range.clone() {
                for z in range.clone() {
                    let chunk_pos = [x + chunk_index[0], y + chunk_index[1], z + chunk_index[2]];
                    if !self.render_chunks.iter().any(|&(pos, _)| pos == chunk_pos) {
                        if world.chunks.get(&chunk_pos).is_some() {
                            let render_chunk = RenderChunk::new(self.facade, world, chunk_pos);
                            self.render_chunks.push((chunk_pos, render_chunk));
                        }
                    }
                }
            }
        }
        for chunk in self.render_chunks.iter_mut() {
            let world_chunk = world.chunks.get(&chunk.0).unwrap();
            if world_chunk.update_render.swap(false, Ordering::Acquire) {
                chunk.1.update(&world, chunk.0);
            }
        }
    }
}