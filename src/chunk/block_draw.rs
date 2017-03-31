use super::direction::Direction;
use super::{ChunkBlockData, Chunk, CHUNK_SIZE};
use self::block_graphics_supplier::*;
use glium::backend::Facade;
use glium::{VertexBuffer, IndexBuffer, Surface, Program, ProgramCreationError, DrawError};
use glium::index::PrimitiveType;
use glium::draw_parameters::DrawParameters;
use glium::uniforms::Sampler;
use glium::texture::CompressedSrgbTexture2dArray;
use std::cell::RefCell;
use std::thread;
use vecmath;
use geometry::CORNER_OFFSET;

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
    pub fn new<BGS: BlockGraphicsSupplier, F: Facade>(facade: &F, chunk: &Chunk, blocks: &BGS, pos: [f32; 3]) -> Self {
        let vertex = Self::get_vertices(&chunk.data, blocks, pos);
        let index = Self::get_indices(vertex.len() / 4);
        RenderChunk {
            v_buf: VertexBuffer::new(facade, &vertex).unwrap(),
            i_buf: IndexBuffer::new(facade, PrimitiveType::TrianglesList, &index).unwrap(),
        }
    }
    pub fn update<BGS: BlockGraphicsSupplier>(&mut self, chunk: &Chunk, blocks: &BGS, pos: [f32; 3]) {
        if chunk.changed.get() {
            let vertex = Self::get_vertices(&chunk.data, blocks, pos);
            let index = Self::get_indices(vertex.len() / 4);
            let facade = self.v_buf.get_context().clone();
            self.v_buf = VertexBuffer::new(&facade, &vertex).unwrap();
            self.i_buf = IndexBuffer::new(&facade, PrimitiveType::TrianglesList, &index).unwrap();
            chunk.changed.set(false);
        }
    }
    pub fn draw<S: Surface>(&self, surface: &mut S, uniforms: &ChunkUniforms, params: &DrawParameters) -> Result<(), DrawError> {
        PROGRAM.with(|prog_cell| {
            let prog_opt = prog_cell.borrow();
            let prog = prog_opt.as_ref().unwrap_or_else(||
                panic!("chunk shader not initialized in thread {:?}", thread::current().name()));
            surface.draw(&self.v_buf, &self.i_buf, prog, &uniform! {matrix:uniforms.transform,light:uniforms.light,sampler:uniforms.sampler}, params)
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
    fn get_vertices<BGS: BlockGraphicsSupplier>(data: &ChunkBlockData, blocks: &BGS, pos: [f32; 3]) -> Vec<Vertex> {
        let mut ver = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                push_row(&mut ver, [x, y, 0], data, Direction::PosX, x == (CHUNK_SIZE - 1), blocks, pos);
                push_row(&mut ver, [x, y, 0], data, Direction::NegX, x == 0, blocks, pos);
            }
        }
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                push_row(&mut ver, [0, y, z], data, Direction::PosZ, z == (CHUNK_SIZE - 1), blocks, pos);
                push_row(&mut ver, [0, y, z], data, Direction::NegZ, z == 0, blocks, pos);
                push_row(&mut ver, [0, y, z], data, Direction::PosY, y == (CHUNK_SIZE - 1), blocks, pos);
                push_row(&mut ver, [0, y, z], data, Direction::NegY, y == 0, blocks, pos);
            }
        }
        ver
    }
}

fn push_row<BGS: BlockGraphicsSupplier>(
    ver: &mut Vec<Vertex>, start: [usize; 3],
    data: &ChunkBlockData,
    direction: Direction,
    face_outside: bool,
    blocks: &BGS,
    chunk_pos: [f32; 3]
) {
    fn get_pos(pos: [usize; 3], chunk_pos: [f32; 3]) -> [f32; 3] {
        [pos[0] as f32 + chunk_pos[0], pos[1] as f32 + chunk_pos[1], pos[2] as f32 + chunk_pos[2]]
    }
    let move_direction = direction_to_move_direction(direction);
    let mut len = 0.;
    let mut texture_id = None;
    for i in 0..CHUNK_SIZE {
        let mut pos = start;
        pos[move_direction] += i;
        let current_block = data[pos[0]][pos[1]][pos[2]];
        let covered = !face_outside && {
            let facing_to = direction.apply_to_pos([pos[0] as i32, pos[1] as i32, pos[2] as i32]);
            blocks.is_opaque(data[facing_to[0] as usize][facing_to[1] as usize][facing_to[2] as usize])
        };
        if let Some(old_texture) = texture_id {
            if covered {
                push_row_face(ver, get_pos(pos, chunk_pos), len, direction, old_texture);
                texture_id = None;
                len = 0.;
            } else {
                match blocks.get_draw_type(current_block) {
                    DrawType::FullOpaqueBlock(new_texture) => {
                        let new_texture = new_texture[direction as usize];
                        if old_texture == new_texture {
                            len += 1.;
                        } else {
                            push_row_face(ver, get_pos(pos, chunk_pos), len, direction, old_texture);
                            texture_id = Some(new_texture);
                            len = 1.;
                        }
                    },
                    DrawType::None => {
                        push_row_face(ver, get_pos(pos, chunk_pos), len, direction, old_texture);
                        texture_id = None;
                        len = 0.
                    }
                }
            }
        } else {
            if !covered {
                match blocks.get_draw_type(current_block) {
                    DrawType::FullOpaqueBlock(texture) => {
                        texture_id = Some(texture[direction as usize]);
                        len = 1.;
                    },
                    DrawType::None => {}
                }
            }
        }
    }
    if let Some(texture) = texture_id {
        let mut pos = get_pos(start, chunk_pos);
        pos[move_direction] += CHUNK_SIZE as f32;
        push_row_face(ver, pos, len, direction, texture);
    }
}

fn direction_to_normal(d: Direction) -> [f32; 3] {
    let normal = d.offset();
    [normal[0] as f32, normal[1] as f32, normal[2] as f32]
}

fn direction_to_corners(d: Direction) -> [usize; 2] {
    match d {
        Direction::PosX => [3, 0],
        Direction::NegX => [1, 2],
        Direction::PosY => [6, 2],
        Direction::NegY => [1, 5],
        Direction::PosZ => [5, 6],
        Direction::NegZ => [2, 1],
    }
}

fn direction_to_move_direction(d: Direction) -> usize {
    match d {
        Direction::PosX => 2,
        Direction::NegX => 2,
        Direction::PosY => 0,
        Direction::NegY => 0,
        Direction::PosZ => 0,
        Direction::NegZ => 0,
    }
}

fn push_row_face(
    ver: &mut Vec<Vertex>,
    end: [f32; 3],
    length: f32,
    face_direction: Direction,
    texture_id: BlockTextureId,
) {
    let corners = direction_to_corners(face_direction);
    let move_direction = direction_to_move_direction(face_direction);
    let mut vertices = [
        (vecmath::vec3_add(end, CORNER_OFFSET[corners[0]]), [0., 0.]),
        (vecmath::vec3_add(end, CORNER_OFFSET[corners[1]]), [1., 0.]),
        (vecmath::vec3_add(end, CORNER_OFFSET[corners[1]]), [1., length]),
        (vecmath::vec3_add(end, CORNER_OFFSET[corners[0]]), [0., length]),
    ];
    vertices[2].0[move_direction] -= length;
    vertices[3].0[move_direction] -= length;
    for v in vertices.iter() {
        ver.push(Vertex {
            position: v.0,
            normal: direction_to_normal(face_direction),
            tex_coords: v.1,
            texture_id: texture_id.to_f32(),
        });
    }
}

thread_local! {
static PROGRAM: RefCell < Option < Program > >= RefCell::new(None)
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    texture_id: f32,
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, normal,texture_id,tex_coords);

const VERTEX_SHADER_SRC: &'static str = r#"
    #version 140

    in vec3 normal;
    in vec3 position;
    in vec2 tex_coords;
    in float texture_id;

    out float brightness;
    out vec2 v_tex_coords;
    out float v_texture_id;

    uniform mat4 matrix;
    uniform vec3 light;

    void main() {
        gl_Position = matrix*vec4(position, 1.0);
        brightness = mix(0.6,1.0,abs(dot(normalize(light),normalize(normal))));
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
        color=texture(sampler,vec3(v_tex_coords,floor(v_texture_id+0.5)));
    }
"#;
