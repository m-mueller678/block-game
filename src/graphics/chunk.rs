use glium::*;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;
use glium::backend::Facade;
use glium::index::PrimitiveType;
use world::{CHUNK_SIZE, World, ChunkReader, chunk_index};
use block::BlockRegistry;
use geometry::*;
use super::DrawType;

use super::{BlockTextureId, QuadVertex};

pub struct RenderChunk {
    v_buf: VertexBuffer<QuadVertex>,
    i_buf: IndexBuffer<u32>,
}

pub struct ChunkUniforms<'a> {
    pub transform: [[f32; 4]; 4],
    pub light: [f32; 3],
    pub sampler: Sampler<'a, CompressedSrgbTexture2dArray>
}

impl RenderChunk {
    pub fn new<F: Facade>(facade: &F, world: &World, pos: [i32; 3]) -> Self {
        let vertex = Self::get_vertices(world, world.blocks(), pos);
        let index = Self::get_indices(vertex.len() / 4);
        RenderChunk {
            v_buf: VertexBuffer::new(facade, &vertex).unwrap(),
            i_buf: IndexBuffer::new(facade, PrimitiveType::TrianglesList, &index).unwrap(),
        }
    }
    pub fn update(&mut self, world: &World, pos: [i32; 3]) {
        let vertex = Self::get_vertices(world, world.blocks(), pos);
        let index = Self::get_indices(vertex.len() / 4);
        let facade = self.v_buf.get_context().clone();
        self.v_buf = VertexBuffer::new(&facade, &vertex).unwrap();
        self.i_buf = IndexBuffer::new(&facade, PrimitiveType::TrianglesList, &index).unwrap();
    }
    pub fn draw<S: Surface>(&self, surface: &mut S, uniforms: &ChunkUniforms, params: &DrawParameters, quad_shader: &Program) -> Result<(), DrawError> {
        surface.draw(&self.v_buf, &self.i_buf, quad_shader, &uniform! {matrix:uniforms.transform,light_direction:uniforms.light,sampler:uniforms.sampler}, params)
    }
    fn get_indices(quad_count: usize) -> Vec<u32> {
        let mut ind = Vec::with_capacity(quad_count * 6);
        for i in 0..(quad_count as u32) {
            ind.push(i * 4 + 0);
            ind.push(i * 4 + 1);
            ind.push(i * 4 + 2);
            ind.push(i * 4 + 0);
            ind.push(i * 4 + 2);
            ind.push(i * 4 + 3);
        }
        ind
    }
    fn get_vertices(world: &World, blocks: &BlockRegistry, pos: [i32; 3]) -> Vec<QuadVertex> {
        let (chunk, adjacent) = Self::lock_chunks(world, pos);
        let mut buffer = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let id = chunk.block(chunk_index(&[x, y, z]));
                    match blocks.draw_type(id) {
                        DrawType::FullOpaqueBlock(textures) => {
                            for d in ALL_DIRECTIONS.iter() {
                                if let (Some(facing_chunk), facing_index) = Self::get_block_at(&chunk, &adjacent, [x, y, z], *d) {
                                    let visible = match blocks.draw_type(facing_chunk.block(facing_index)) {
                                        DrawType::FullOpaqueBlock(_) => false,
                                        DrawType::None => true,
                                    };
                                    if visible {
                                        let float_pos = [
                                            pos[0] as f32 * CHUNK_SIZE as f32 + x as f32,
                                            pos[1] as f32 * CHUNK_SIZE as f32 + y as f32,
                                            pos[2] as f32 * CHUNK_SIZE as f32 + z as f32
                                        ];
                                        Self::push_face(&mut buffer, float_pos, *d, textures[*d as usize], facing_chunk.light(facing_index));
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
    fn lock_chunks(world: &World, pos: [i32; 3]) -> (ChunkReader, [Option<ChunkReader>; 6]) {
        let l1 = world.lock_chunk(&[pos[0] - 1, pos[1], pos[2]]);
        let l2 = world.lock_chunk(&[pos[0], pos[1] - 1, pos[2]]);
        let l3 = world.lock_chunk(&[pos[0], pos[1], pos[2] - 1]);
        let l4 = world.lock_chunk(&[pos[0], pos[1], pos[2]]).expect("RenderChunk: chunk does not exist");
        let l5 = world.lock_chunk(&[pos[0], pos[1], pos[2] + 1]);
        let l6 = world.lock_chunk(&[pos[0], pos[1] + 1, pos[2]]);
        let l7 = world.lock_chunk(&[pos[0] + 1, pos[1], pos[2]]);
        (l4, [l7, l1, l6, l2, l5, l3])
    }
    fn push_face(buffer: &mut Vec<QuadVertex>, pos: [f32; 3], direction: Direction, texture: BlockTextureId, light: u8) {
        use vecmath::vec3_add;
        let vertices = CUBE_FACES[direction as usize];
        let tex_coords = [[0., 0.], [0., 1.], [1., 1.], [1., 0.]];
        for i in 0..4 {
            let normal = direction.offset();
            buffer.push(QuadVertex {
                position: vec3_add(pos, CORNER_OFFSET[vertices[i]]),
                normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32],
                tex_coords: tex_coords[i],
                texture_id: texture.to_f32(),
                light_level: light as f32 / 15.,
            });
        }
    }
    fn get_block_at<'a, 'b>(chunk: &'b ChunkReader<'a>, adjacent: &'b [Option<ChunkReader<'a>>; 6], mut pos: [usize; 3], d: Direction) -> (Option<&'b ChunkReader<'a>>, usize) {
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
        (if outside { adjacent[d as usize].as_ref() } else { Some(chunk) }, chunk_index(&pos))
    }
}