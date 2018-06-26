use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver, TryRecvError};
use std::thread;
use std::borrow::Cow;
use std::rc::Rc;
use time::{SteadyTime, Duration};
use vecmath::*;
use glium;
use glium::backend::Facade;
use glium::{VertexBuffer, vertex, IndexBuffer, ProgramCreationError, Program, Surface,
            DrawParameters};
use glium::index::PrimitiveType;
use geometry::{Direction, CORNER_OFFSET, CUBE_FACES};
use world::{BlockPos, World};
use graphics::block::DrawType;
use super::quad;

pub struct Overlay2d<O, P>
where
    O: FnMut(i32, i32) -> [f32; 3] + Send + 'static,
    P: FnMut() -> BlockPos + Send + 'static,
{
    data: O,
    player_position: P,
    range: i32,
    world: Arc<World>,
}

impl<O, P> Overlay2d<O, P>
where
    O: FnMut(i32, i32) -> [f32; 3] + Send + 'static,
    P: FnMut() -> BlockPos + Send + 'static,
{
    pub fn new(overlay: O, player_pos: P, range: i32, world: Arc<World>) -> Self {
        Overlay2d {
            data: overlay,
            player_position: player_pos,
            range: range,
            world: world,
        }
    }
}

impl<O, P> OverlayDataSupplier for Overlay2d<O, P>
where
    O: FnMut(i32, i32) -> [f32; 3] + Send + 'static,
    P: FnMut() -> BlockPos + Send + 'static,
{
    fn get_data(&mut self) -> Vec<(BlockPos, Direction, [f32; 3])> {
        let pos = (self.player_position)();
        let mut faces = Vec::with_capacity((self.range as usize * 2 + 1).pow(2));
        for x in (pos[0] - self.range)..(pos[0] + self.range) {
            for z in (pos[2] - self.range)..(pos[2] + self.range) {
                for dy in 0..self.range {
                    match self.world.get_block(BlockPos([x, pos[1] - dy, z])).map(|id| {
                        self.world.game_data().blocks().draw_type(id)
                    }) {
                        None |
                        Some(DrawType::None) => {}
                        Some(DrawType::FullOpaqueBlock(..)) => {
                            faces.push((
                                BlockPos([x, pos[1] - dy, z]),
                                Direction::PosY,
                                (self.data)(x, z),
                            ));
                            break;
                        }
                    }
                }
            }
        }
        faces
    }
}

pub fn load_overlay_shader<F: Facade>(f: &F) -> Result<Program, ProgramCreationError> {
    Program::from_source(f, VERTEX_SRC, FRAGMENT_SRC, None)
}

pub trait OverlayDataSupplier
where
    Self: Send + 'static,
{
    fn get_data(&mut self) -> Vec<(BlockPos, Direction, [f32; 3])>;
}

impl<T> OverlayDataSupplier for T where Self: FnMut() -> Vec<(BlockPos, Direction, [f32; 3])> + Send + 'static {
    fn get_data(&mut self) -> Vec<(BlockPos, Direction, [f32; 3])> {
        self()
    }
}

pub struct BlockOverlay {
    v_buf: VertexBuffer<Vertex>,
    i_buf: IndexBuffer<u32>,
    receiver: Receiver<Vec<Vertex>>,
}

#[derive(Debug)]
pub enum OverlayDrawError {
    BufferCreationError(Box<Error>),
    DrawError(glium::DrawError),
    OverlayPanic,
}

impl BlockOverlay {
    pub fn new<F: Facade>(overlay: Box<OverlayDataSupplier>, facade: &F) -> Self {
        BlockOverlay {
            v_buf: VertexBuffer::<Vertex>::new(facade, &[]).unwrap(),
            i_buf: IndexBuffer::<u32>::new(facade, PrimitiveType::TrianglesList, &[]).unwrap(),
            receiver: start_overlay(overlay),
        }
    }
    pub fn draw<S: Surface>(
        &mut self,
        surface: &mut S,
        shader: &Program,
        transform: [[f32; 4]; 4],
    ) -> Result<(), OverlayDrawError> {
        match self.receiver.try_recv() {
            Err(TryRecvError::Disconnected) => {
                return Err(OverlayDrawError::OverlayPanic);
            }
            Err(TryRecvError::Empty) => {}
            Ok(vertices) => {
                let context = Rc::clone(self.v_buf.get_context());
                let v_buf = match VertexBuffer::new(&context, &vertices) {
                    Err(e) => {
                        return Err(OverlayDrawError::BufferCreationError(Box::new(e)));
                    }
                    Ok(buffer) => buffer,
                };
                let indices = quad::get_triangle_indices(vertices.len() / 4);
                let i_buf =
                    match IndexBuffer::new(&context, PrimitiveType::TrianglesList, &indices) {
                        Err(e) => {
                            return Err(OverlayDrawError::BufferCreationError(Box::new(e)));
                        }
                        Ok(buffer) => buffer,
                    };

                self.v_buf = v_buf;
                self.i_buf = i_buf;
            }
        }
        let parameters = DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLessOrEqual,
                write: false,
                ..Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };
        if let Err(e) = surface.draw(
            &self.v_buf,
            &self.i_buf,
            shader,
            &uniform! {transform:transform},
            &parameters,
        )
        {
            return Err(OverlayDrawError::DrawError(e));
        }
        Ok(())
    }
}

fn start_overlay(overlay: Box<OverlayDataSupplier>) -> Receiver<Vec<Vertex>> {
    let (sender, receiver) = sync_channel(1);
    thread::Builder::new()
        .name("block overlay updater".into())
        .spawn(move || update_overlay(overlay, sender))
        .unwrap();
    receiver
}

fn update_overlay(mut overlay: Box<OverlayDataSupplier>, sender: SyncSender<Vec<Vertex>>) {
    let mut scheduled_end = SteadyTime::now() + Duration::milliseconds(20);
    loop {
        let faces = overlay.get_data();
        let mut vertices = Vec::with_capacity(faces.len() * 4);
        for (pos, face, color) in faces {
            let pos = [pos[0] as f32, pos[1] as f32, pos[2] as f32];
            for i in 0..4 {
                let corner = vec3_add(
                    vec3_scale(CORNER_OFFSET[CUBE_FACES[face as usize][i]], 1.05),
                    [-0.025; 3],
                );
                vertices.push(Vertex {
                    position: vec3_add(pos, corner),
                    color: color,
                });
            }
        }
        if sender.send(vertices).is_err() {
            drop(sender);
            return;
        }
        let end_time = SteadyTime::now();
        if end_time < scheduled_end {
            thread::sleep((scheduled_end - end_time).to_std().unwrap());
            scheduled_end = scheduled_end + Duration::milliseconds(20);
        } else {
            scheduled_end = end_time + Duration::milliseconds(20);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

//workaround for bug in implement_vertex macro
//glium issue #1607
impl vertex::Vertex for Vertex {
    fn build_bindings() -> vertex::VertexFormat {
        static VERTEX_FORMAT: [(Cow<'static, str>, usize, vertex::AttributeType, bool); 2] = [
            (
                Cow::Borrowed("position"),
                0,
                vertex::AttributeType::F32F32F32,
                false,
            ),
            (
                Cow::Borrowed("color"),
                4 * 3,
                vertex::AttributeType::F32F32F32,
                false,
            ),
        ];
        Cow::Borrowed(&VERTEX_FORMAT)
    }
}

const VERTEX_SRC: &str = r#"
#version 140

in vec3 position;
in vec3 color;

out vec3 overlay_color;

uniform mat4 transform;

void main(){
    gl_Position=transform*vec4(position,1.0);
    overlay_color=color;
}
"#;

const FRAGMENT_SRC: &str = r#"
#version 140

in vec3 overlay_color;

out vec4 color;

void main(){
    color=vec4(overlay_color.rgb,0.5);
}
"#;
