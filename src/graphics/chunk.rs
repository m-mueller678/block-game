use std::sync::{Weak, Arc};
use glium::*;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;
use glium::backend::Facade;
use glium::index::PrimitiveType;
use world::{CHUNK_SIZE, ChunkPos, Chunk};
use block::BlockRegistry;
use geometry::*;
use graphics::chunk_update::ChunkRegion;
use super::DrawType;
use super::quad;

use super::{TextureId, QuadVertex};

pub struct RenderChunk {
    v_buf: VertexBuffer<QuadVertex>,
    i_buf: IndexBuffer<u32>,
}

pub struct RenderChunkData {
    v_buf: Vec<QuadVertex>,
    i_buf: Vec<u32>,
}

pub struct ChunkUniforms<'a> {
    pub transform: [[f32; 4]; 4],
    pub light: [f32; 3],
    pub sampler: Sampler<'a, CompressedSrgbTexture2dArray>,
}

impl RenderChunkData {
    pub fn new(chunk: &ChunkRegion, blocks: &BlockRegistry, pos: ChunkPos) -> Self {
        let vertex = Self::get_vertices(chunk, blocks, pos);
        let index = quad::get_triangle_indices(vertex.len() / 4);
        RenderChunkData {
            v_buf: vertex,
            i_buf: index,
        }
    }

    fn get_vertices(
        region: &ChunkRegion,
        blocks: &BlockRegistry,
        pos: ChunkPos,
    ) -> Vec<QuadVertex> {
        let adjacent = [
            Weak::upgrade(&region.neighbours[0]),
            Weak::upgrade(&region.neighbours[1]),
            Weak::upgrade(&region.neighbours[2]),
            Weak::upgrade(&region.neighbours[3]),
            Weak::upgrade(&region.neighbours[4]),
            Weak::upgrade(&region.neighbours[5]),
        ];
        let chunk = &*region.center;
        let mut buffer = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let id = chunk.data[[x, y, z]].load();
                    match blocks.draw_type(id) {
                        DrawType::FullOpaqueBlock(textures) => {
                            for d in &ALL_DIRECTIONS {
                                if let Some((facing_chunk, facing_index)) =
                                Self::get_block_at(&*chunk, &adjacent, [x, y, z], *d)
                                    {
                                        let visible =
                                            match blocks.draw_type(facing_chunk.data[facing_index].load()) {
                                                DrawType::FullOpaqueBlock(_) => false,
                                                DrawType::None => true,
                                            };
                                        if visible {
                                            let float_pos =
                                                [
                                                    pos[0] as f32 * CHUNK_SIZE as f32 + x as f32,
                                                    pos[1] as f32 * CHUNK_SIZE as f32 + y as f32,
                                                    pos[2] as f32 * CHUNK_SIZE as f32 + z as f32,
                                                ];
                                            Self::push_face(
                                                &mut buffer,
                                                float_pos,
                                                *d,
                                                textures[*d as usize],
                                                facing_chunk.effective_light(facing_index),
                                            );
                                        }
                                    }
                            }
                        }
                        DrawType::None => {}
                    }
                }
            }
        }
        buffer
    }
    fn push_face(
        buffer: &mut Vec<QuadVertex>,
        pos: [f32; 3],
        direction: Direction,
        texture: TextureId,
        light: u8,
    ) {
        use vecmath::vec3_add;
        let vertices = CUBE_FACES[direction as usize];
        let tex_coords = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        for i in 0..4 {
            let normal = direction.offset();
            buffer.push(QuadVertex {
                position: vec3_add(pos, CORNER_OFFSET[vertices[i]]),
                normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32],
                tex_coords: tex_coords[i],
                texture_id: texture.to_u32() as f32,
                light_level: f32::from(light) / 15.,
            });
        }
    }
    fn get_block_at<'a>(
        chunk: &'a Chunk,
        adjacent: &'a [Option<Arc<Chunk>>; 6],
        mut pos: [usize; 3],
        d: Direction,
    ) -> Option<(&'a Chunk, [usize; 3])> {
        let outside = match d {
            Direction::PosX => pos[0] + 1 == CHUNK_SIZE,
            Direction::PosY => pos[1] + 1 == CHUNK_SIZE,
            Direction::PosZ => pos[2] + 1 == CHUNK_SIZE,
            Direction::NegX => pos[0] == 0,
            Direction::NegY => pos[1] == 0,
            Direction::NegZ => pos[2] == 0,
        };
        match d {
            Direction::PosX => pos[0] = (pos[0] + 1) % CHUNK_SIZE,
            Direction::PosY => pos[1] = (pos[1] + 1) % CHUNK_SIZE,
            Direction::PosZ => pos[2] = (pos[2] + 1) % CHUNK_SIZE,
            Direction::NegX => pos[0] = (pos[0] + CHUNK_SIZE - 1) % CHUNK_SIZE,
            Direction::NegY => pos[1] = (pos[1] + CHUNK_SIZE - 1) % CHUNK_SIZE,
            Direction::NegZ => pos[2] = (pos[2] + CHUNK_SIZE - 1) % CHUNK_SIZE,
        };
        if outside {
            adjacent[d as usize].as_ref().map(|arc| (&**arc, pos))
        } else {
            Some((chunk, pos))
        }
    }
}

impl RenderChunk {
    pub fn draw<S: Surface>(
        &self,
        surface: &mut S,
        uniforms: &ChunkUniforms,
        params: &DrawParameters,
        quad_shader: &Program,
    ) -> Result<(), DrawError> {
        surface.draw(&self.v_buf, &self.i_buf, quad_shader, &uniform! {matrix:uniforms.transform,light_direction:uniforms.light,sampler:uniforms.sampler}, params)
    }
    pub fn new<F: Facade>(data: RenderChunkData, facade: &F) -> Self {
        RenderChunk {
            v_buf: VertexBuffer::new(facade, &data.v_buf).unwrap(),
            i_buf: IndexBuffer::new(facade, PrimitiveType::TrianglesList, &data.i_buf).unwrap(),
        }
    }
}
