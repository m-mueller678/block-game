use geometry::direction::{Direction, ALL_DIRECTIONS};
use super::{Chunk, CHUNK_SIZE};
use self::block_graphics_supplier::*;
use glium::backend::Facade;
use glium::{VertexBuffer, IndexBuffer, Surface, Program, ProgramCreationError, DrawError};
use glium::index::PrimitiveType;
use glium::draw_parameters::DrawParameters;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;
use std::cell::RefCell;
use std::thread;
use geometry::{CORNER_OFFSET, CUBE_FACES};
use world::{World, ChunkReader};

pub mod block_graphics_supplier {
    use block::BlockId;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct BlockTextureId(u32);

    impl BlockTextureId {
        pub fn new(i: u32) -> Self {
            BlockTextureId(i)
        }
        pub fn to_f32(&self) -> f32 {
            self.0 as f32
        }
    }

    #[derive(Clone)]
    pub enum DrawType {
        FullOpaqueBlock([BlockTextureId; 6]),
        None
    }

    pub trait BlockGraphicsSupplier {
        fn get_draw_type(&self, block_id: BlockId) -> DrawType;
        fn is_opaque(&self, block_id: BlockId) -> bool;
    }
}

pub struct RenderChunk {
    v_buf: VertexBuffer<Vertex>,
    i_buf: IndexBuffer<u32>,
}

pub struct ChunkUniforms<'a> {
    pub transform: [[f32; 4]; 4],
    pub light: [f32; 3],
    pub sampler: Sampler<'a, CompressedSrgbTexture2dArray>
}

pub fn init_chunk_shader<F: Facade>(facade: &F) -> Result<(), ProgramCreationError> {
    PROGRAM.with(|prog_cell| {
        let mut prog_opt = prog_cell.borrow_mut();
        if prog_opt.is_none() {
            let prog = Program::from_source(facade, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None)?;
            *prog_opt = Some(prog);
            Ok(())
        } else {
            panic!("chunk shader already initialized in thread {:?}", thread::current().name());
        }
    })
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
    pub fn draw<S: Surface>(&self, surface: &mut S, uniforms: &ChunkUniforms, params: &DrawParameters) -> Result<(), DrawError> {
        PROGRAM.with(|prog_cell| {
            let prog_opt = prog_cell.borrow();
            let prog = prog_opt.as_ref().unwrap_or_else(||
                panic!("chunk shader not initialized in thread {:?}", thread::current().name()));
            surface.draw(&self.v_buf, &self.i_buf, prog, &uniform! {matrix:uniforms.transform,light_direction:uniforms.light,sampler:uniforms.sampler}, params)
        })
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
    fn get_vertices<BGS: BlockGraphicsSupplier>(world: &World, blocks: &BGS, pos: [i32; 3]) -> Vec<Vertex> {
        let (chunk, adjacent) = Self::lock_chunks(world, pos);
        let mut buffer = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let id = chunk.data[Chunk::u_index(&[x, y, z])];
                    match blocks.get_draw_type(id) {
                        DrawType::FullOpaqueBlock(textures) => {
                            for d in ALL_DIRECTIONS.iter() {
                                if let (Some(facing_chunk), facing_index) = Self::get_block_at(&chunk, &adjacent, [x, y, z], *d) {
                                    let visible = match blocks.get_draw_type(facing_chunk.data[facing_index]) {
                                        DrawType::FullOpaqueBlock(_) => false,
                                        DrawType::None => true,
                                    };
                                    if visible {
                                        let float_pos = [
                                            pos[0] as f32 * CHUNK_SIZE as f32 + x as f32,
                                            pos[1] as f32 * CHUNK_SIZE as f32 + y as f32,
                                            pos[2] as f32 * CHUNK_SIZE as f32 + z as f32
                                        ];
                                        Self::push_face(&mut buffer, float_pos, *d, textures[*d as usize], facing_chunk.light[facing_index].0);
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
    fn push_face(buffer: &mut Vec<Vertex>, pos: [f32; 3], direction: Direction, texture: BlockTextureId, light: u8) {
        use vecmath::vec3_add;
        let vertices = CUBE_FACES[direction as usize];
        let tex_coords = [[0., 0.], [0., 1.], [1., 1.], [1., 0.]];
        for i in 0..4 {
            let normal = direction.offset();
            buffer.push(Vertex {
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
        (if outside { adjacent[d as usize].as_ref() } else { Some(chunk) }, Chunk::u_index(&pos))
    }
}



thread_local! {
static PROGRAM: RefCell < Option < Program > >= RefCell::new(None)
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
    texture_id: f32,
    light_level: f32,
}

implement_vertex!(Vertex, position, normal,texture_id,tex_coords,light_level);

const VERTEX_SHADER_SRC: &'static str = r#"
    #version 140

    in vec3 normal;
    in vec3 position;
    in vec2 tex_coords;
    in float texture_id;
    in float light_level;

    out float brightness;
    out vec2 v_tex_coords;
    out float v_texture_id;

    uniform mat4 matrix;
    uniform vec3 light_direction;

    void main() {
        gl_Position = matrix*vec4(position, 1.0);
        brightness = mix(0.6,1.0,abs(dot(normalize(light_direction),normalize(normal))))*light_level;
        v_tex_coords=tex_coords;
        v_texture_id=texture_id;
    }
"#;

const FRAGMENT_SHADER_SRC: &'static str = r#"
    #version 140

    in float brightness;
    in vec2 v_tex_coords;
    in float v_texture_id;

    out vec4 color;

    uniform sampler2DArray sampler;


    void main() {
        color=texture(sampler,vec3(v_tex_coords,floor(v_texture_id+0.5)))*brightness
        ;
    }
"#;
