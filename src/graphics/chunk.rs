use std::rc::Rc;
use glium::*;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;
use glium::backend::Facade;
use glium::index::PrimitiveType;
use world::{CHUNK_SIZE, ChunkReader, ChunkPos, ChunkMap};
use block::BlockRegistry;
use geometry::*;
use super::DrawType;
use super::quad;

use super::{TextureId, QuadVertex};

pub struct RenderChunkUpdate {
    vbuf: Vec<QuadVertex>,
    ibuf: Vec<u32>,
}

impl RenderChunkUpdate {
    pub fn new(world: &ChunkMap, pos: ChunkPos) -> Self {
        let vbuf = Self::get_vertices(world, world.game_data().blocks(), pos);
        let ibuf = quad::get_triangle_indices(vbuf.len() / 4);
        RenderChunkUpdate { vbuf, ibuf }
    }
    fn get_vertices(
        world: &ChunkMap,
        blocks: &BlockRegistry,
        pos: ChunkPos,
    ) -> Vec<QuadVertex> {
        let (chunk, adjacent) ={
            if let Some(locks)=Self::try_lock_chunks(world, pos){
                locks
            }else{
                return Vec::new();
            }
        };
        let mut buffer = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let id = chunk.block([x, y, z]);
                    match blocks.draw_type(id) {
                        DrawType::FullOpaqueBlock(textures) => {
                            for d in &ALL_DIRECTIONS {
                                if let (Some(facing_chunk), facing_index) =
                                Self::get_block_at(&chunk, &adjacent, [x, y, z], *d)
                                    {
                                        let visible =
                                            match blocks.draw_type(facing_chunk.block(facing_index)) {
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
    fn try_lock_chunks<'a>(
        world: &'a ChunkMap,
        pos: ChunkPos,
    ) -> Option<(ChunkReader<'a>, [Option<ChunkReader<'a>>; 6])> {
        let l1 = world.lock_chunk(pos.facing(Direction::NegX));
        let l2 = world.lock_chunk(pos.facing(Direction::NegY));
        let l3 = world.lock_chunk(pos.facing(Direction::NegZ));
        let l4 = if let Some(c) = world.lock_chunk(pos) {
            c
        } else {
            return None
        };
        let l5 = world.lock_chunk(pos.facing(Direction::PosZ));
        let l6 = world.lock_chunk(pos.facing(Direction::PosY));
        let l7 = world.lock_chunk(pos.facing(Direction::PosX));
        Some((l4, [l7, l1, l6, l2, l5, l3]))
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
    fn get_block_at<'a, 'b>(
        chunk: &'b ChunkReader<'a>,
        adjacent: &'b [Option<ChunkReader<'a>>; 6],
        mut pos: [usize; 3],
        d: Direction,
    ) -> (Option<&'b ChunkReader<'a>>, [usize; 3]) {
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
        (
            if outside {
                adjacent[d as usize].as_ref()
            } else {
                Some(chunk)
            },
            pos,
        )
    }

    pub fn is_empty(&self)->bool{
        self.vbuf.is_empty()
    }
}

pub struct RenderChunk {
    v_buf: VertexBuffer<QuadVertex>,
    i_buf: IndexBuffer<u32>,
}

pub struct ChunkUniforms<'a> {
    pub transform: [[f32; 4]; 4],
    pub light: [f32; 3],
    pub sampler: Sampler<'a, CompressedSrgbTexture2dArray>,
}

impl RenderChunk {
    pub fn from_update<F: Facade>(facade: &F, update: RenderChunkUpdate) -> Self {
        RenderChunk {
            v_buf: VertexBuffer::new(facade, &update.vbuf).unwrap(),
            i_buf: IndexBuffer::new(facade, PrimitiveType::TrianglesList, &update.ibuf).unwrap(),
        }
    }

    pub fn apply_update(&mut self, update: RenderChunkUpdate) {
        let facade = Rc::clone(self.v_buf.get_context());
        *self = Self::from_update(&facade, update);
    }
    pub fn draw<S: Surface>(
        &self,
        surface: &mut S,
        uniforms: &ChunkUniforms,
        params: &DrawParameters,
        quad_shader: &Program,
    ) -> Result<(), DrawError> {
        surface.draw(&self.v_buf, &self.i_buf, quad_shader, &uniform! {matrix:uniforms.transform,light_direction:uniforms.light,sampler:uniforms.sampler}, params)
    }
}
