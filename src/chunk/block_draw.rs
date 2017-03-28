use super::direction::Direction;
use super::{ChunkBlockData, Chunk, CHUNK_SIZE};
use glium::backend::Facade;
use glium::{VertexBuffer, IndexBuffer, Surface, Program, ProgramCreationError, DrawError};
use glium::index::PrimitiveType;
use glium::draw_parameters::DrawParameters;
use std::cell::RefCell;
use std::thread;
use vecmath;

pub type BlockTextureId = u32;

pub trait BlockGraphicsSupplier {
    fn get_texture(&self, block_id: u32, Direction) -> BlockTextureId;
    fn is_opaque(&self, block_id: u32) -> bool;
}

pub struct RenderChunk {
    v_buf: VertexBuffer<Vertex>,
    i_buf: IndexBuffer<u32>,
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
        let vertex = Self::get_vertices(&chunk.data, blocks, pos);
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
            surface.draw(&self.v_buf, &self.i_buf, prog, &uniform! {matrix:uniforms.transform,light:uniforms.light}, params)
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

fn push_row<BGS: BlockGraphicsSupplier>(ver: &mut Vec<Vertex>, start: [usize; 3], data: &ChunkBlockData, direction: Direction, face_outside: bool, blocks: &BGS, chunk_pos: [f32; 3]) {
    let move_direction = direction_to_move_direction(direction);
    let mut len = 0.;
    let mut block_id = None;
    for i in 0..CHUNK_SIZE {
        let mut pos = start;
        pos[move_direction] += i;
        let current_block = data[pos[0]][pos[1]][pos[2]];
        let covered = !face_outside && {
            let facing_to = direction.apply_to_pos([pos[0] as i32, pos[1] as i32, pos[2] as i32]);
            blocks.is_opaque(data[facing_to[0] as usize][facing_to[1] as usize][facing_to[2] as usize])
        };
        if let Some(old_block) = block_id {
            if covered {
                push_row_face(ver, [pos[0] as f32 + chunk_pos[0], pos[1] as f32 + chunk_pos[1], pos[2] as f32 + chunk_pos[2]], len, direction);
                block_id = None;
                len = 0.;
            } else {
                if old_block == current_block {
                    len += 1.;
                } else {
                    push_row_face(ver, [pos[0] as f32 + chunk_pos[0], pos[1] as f32 + chunk_pos[1], pos[2] as f32 + chunk_pos[2]], len, direction);
                    block_id = if blocks.is_opaque(current_block) { Some(current_block) } else { None };
                    len = 1.;
                }
            }
        } else {
            if !covered && blocks.is_opaque(current_block) {
                block_id = Some(current_block);
                len = 1.;
            }
        }
    }
    if block_id.is_some() {
        let mut pos = [chunk_pos[0] + start[0] as f32, chunk_pos[1] + start[1] as f32, chunk_pos[2] + start[2] as f32];
        pos[move_direction] += CHUNK_SIZE as f32;
        push_row_face(ver, pos, len, direction);
    }
}

fn direction_to_normal(d: Direction) -> [f32; 3] {
    let normal = d.offset();
    [normal[0] as f32, normal[1] as f32, normal[2] as f32]
}

const CORNER_OFFSET: [[f32; 3]; 8] = [
    [1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.],
    [1., 0., 1.], [0., 0., 1.], [0., 1., 1.], [1., 1., 1.],
];

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

fn push_row_face(ver: &mut Vec<Vertex>, end: [f32; 3], length: f32, face_direction: Direction) {
    let corners = direction_to_corners(face_direction);
    let move_direction = direction_to_move_direction(face_direction);
    let mut vertices = [
        vecmath::vec3_add(end, CORNER_OFFSET[corners[0]]),
        vecmath::vec3_add(end, CORNER_OFFSET[corners[1]]),
        vecmath::vec3_add(end, CORNER_OFFSET[corners[1]]),
        vecmath::vec3_add(end, CORNER_OFFSET[corners[0]]),
    ];
    vertices[2][move_direction] -= length;
    vertices[3][move_direction] -= length;
    for v in vertices.iter() {
        ver.push(Vertex { position: *v, normal: direction_to_normal(face_direction) });
    }
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

pub struct ChunkUniforms {
    pub transform: [[f32; 4]; 4],
    pub light: [f32; 3],
}

thread_local! {
static PROGRAM: RefCell < Option < Program > >= RefCell::new(None)
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

implement_vertex!(Vertex, position, normal);

const VERTEX_SHADER_SRC: &'static str = r#"
    #version 140

    in vec3 normal;
    in vec3 position;

    out float brightness;

    uniform mat4 matrix;
    uniform vec3 light;

    void main() {
        gl_Position = matrix*vec4(position, 1.0);
        brightness = mix(0.6,1.0,abs(dot(normalize(light),normalize(normal))));
    }
"#;

const FRAGMENT_SHADER_SRC: &'static str = r#"
    #version 140

    in float brightness;

    out vec4 color;

    void main() {
        color=vec4(brightness,0.,0.,1.);
    }
"#;
