pub use self::generator::Generator;
use block::{BlockId, BlockRegistry};
use std::collections::hash_map::{HashMap};
use chunk::{Chunk, CHUNK_SIZE, RenderChunk, ChunkUniforms};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use glium;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;

mod generator;

pub struct World {
    chunks: HashMap<[i32; 3], ChunkData>,
    inserter: Mutex<(Generator, Vec<([i32; 3], Chunk)>)>,
    blocks: Arc<BlockRegistry>,
}

pub enum ChunkAccessError {
    NoChunk
}

struct ChunkData {
    chunk: Mutex<Chunk>,
    update_render: AtomicBool,
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
        use std::mem::replace;
        let (_, ref mut buffer) = *self.inserter.get_mut().unwrap();
        let buffer = replace(buffer, Vec::new());
        for (pos, chunk) in buffer.into_iter() {
            self.chunks.insert(pos, ChunkData {
                chunk: Mutex::new(chunk),
                update_render: AtomicBool::new(false)
            });
        }
    }
    pub fn set_block(&self, pos: &[i32; 3], block: BlockId) -> Result<(), ChunkAccessError> {
        use num::Integer;
        let chunk_size = CHUNK_SIZE as i32;
        let chunk_pos = [
            pos[0].mod_floor(&chunk_size) as usize,
            pos[1].mod_floor(&chunk_size) as usize,
            pos[2].mod_floor(&chunk_size) as usize,
        ];
        if let Some(chunk) = self.chunks.get(&Self::chunk_at(pos)) {
            chunk.chunk.lock().unwrap().set_block(&chunk_pos, block);
            chunk.update_render.store(true, Ordering::Relaxed);
            Ok(())
        } else {
            Err(ChunkAccessError::NoChunk)
        }
    }
    pub fn new(blocks: Arc<BlockRegistry>, generator: Generator) -> Self {
        World {
            chunks: HashMap::new(),
            blocks: blocks,
            inserter: Mutex::new((generator, Vec::new())),
        }
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
            if !buffer.iter().any(|&(p, _)| p == *pos) {
                buffer.push((*pos, generator.gen_chunk(pos)));
            }
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
                && corners.iter().any(|c| c[0] < 1.)
                && corners.iter().any(|c| c[0] > -1.)
                && corners.iter().any(|c| c[1] < 1.)
                && corners.iter().any(|c| c[1] > -1.)
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
                        if let Some(chunk) = world.chunks.get(&chunk_pos) {
                            let fpos = [
                                chunk_pos[0] as f32 * CHUNK_SIZE as f32,
                                chunk_pos[1] as f32 * CHUNK_SIZE as f32,
                                chunk_pos[2] as f32 * CHUNK_SIZE as f32
                            ];
                            let render_chunk = RenderChunk::new(self.facade, &*chunk.chunk.lock().unwrap(), &*world.blocks, fpos);
                            self.render_chunks.push((chunk_pos, render_chunk));
                        }
                    }
                }
            }
        }
        for chunk in self.render_chunks.iter_mut() {
            let fpos = [
                chunk.0[0] as f32 * CHUNK_SIZE as f32,
                chunk.0[1] as f32 * CHUNK_SIZE as f32,
                chunk.0[2] as f32 * CHUNK_SIZE as f32
            ];
            let world_chunk = world.chunks.get(&chunk.0).unwrap();
            if world_chunk.update_render.swap(false, Ordering::Relaxed) {
                chunk.1.update(&*world_chunk.chunk.lock().unwrap(), &*world.blocks, fpos);
            }
        }
    }
}