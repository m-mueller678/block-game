pub use self::generator::Generator;
use block::{BlockId, BlockRegistry};
use std::collections::hash_map::{HashMap};
use chunk::{Chunk, CHUNK_SIZE, RenderChunk, ChunkUniforms};
use glium;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;

mod generator;

pub struct World<'a> {
    chunks: HashMap<[i32; 3], Chunk>,
    blocks: &'a BlockRegistry,
    generator: Generator
}


impl<'a> World<'a> {
    pub fn gen_area(&mut self, pos: &[i32; 3], range: i32) {
        let base = Self::chunk_at(pos);
        for x in (base[0] - range)..(base[0] + range + 1) {
            for y in (base[1] - range)..(base[1] + range + 1) {
                for z in (base[2] - range)..(base[2] + range + 1) {
                    self.create_chunk(&[x, y, z]);
                }
            }
        }
    }
    pub fn set_block(&mut self, pos: &[i32; 3], block: BlockId) {
        use num::Integer;
        let chunk_size = CHUNK_SIZE as i32;
        let chunk_pos = [
            pos[0].mod_floor(&chunk_size) as usize,
            pos[1].mod_floor(&chunk_size) as usize,
            pos[2].mod_floor(&chunk_size) as usize,
        ];
        self.create_chunk(&Self::chunk_at(pos)).set_block(&chunk_pos, block);
    }
    pub fn new(blocks: &'a BlockRegistry, generator: Generator) -> Self {
        World {
            chunks: HashMap::new(),
            blocks: blocks,
            generator: generator,
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
    fn create_chunk(&mut self, pos: &[i32; 3]) -> &mut Chunk {
        let gen = &mut self.generator;
        self.chunks.entry(*pos).or_insert_with(|| gen.gen_chunk(pos))
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
                            let render_chunk = RenderChunk::new(self.facade, &chunk, world.blocks, fpos);
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
            chunk.1.update(world.chunks.get(&chunk.0).unwrap(), world.blocks, fpos);
        }
    }
}