use glium::texture::CompressedSrgbTexture2dArray;
use glium::backend::glutin_backend::GlutinFacade;
use glium::*;
use glium::uniforms::SamplerWrapFunction;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use graphics::*;
use world::{BlockPos, World};
use cam::Camera;
use geometry::*;
use vecmath::{vec3_add, vec3_scale, col_mat4_mul};
use window_util;

pub enum Message {
    CamChanged { pos: [f32; 3], direction: [f32; 3] },
    MousePressed { button: usize },
    MouseReleased { button: usize },
    BlockTargetChanged { target: Option<ray::BlockIntersection> },
}


pub struct Ui {
    display: GlutinFacade,
    quad_shader: Program,
    line_shader: Program,
    event_sender: Sender<Message>,
    textures: CompressedSrgbTexture2dArray,
    world: Arc<World>,
    camera: Camera,
    world_render: WorldRender,
    cursor_line_vertices: VertexBuffer<LineVertex>,
    cursor_line_indices: IndexBuffer<u32>,
    yaw: f32,
    pitch: f32,
    block_target: Option<ray::BlockIntersection>,
}

impl Ui {
    pub fn new(
        display: GlutinFacade,
        quad_shader: Program,
        line_shader: Program,
        event_sender: Sender<Message>,
        textures: CompressedSrgbTexture2dArray,
        world: Arc<World>,
    ) -> Self {
        let index_buffer = IndexBuffer::<u32>::new
            (&display, index::PrimitiveType::LinesList, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
        let vertex_buffer = VertexBuffer::new(&display, &
            [LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] }; 10]).unwrap();
        Ui {
            display: display,
            quad_shader: quad_shader,
            line_shader: line_shader,
            event_sender: event_sender,
            textures: textures,
            world: world,
            camera: Camera::new([0., 100., 0.]),
            world_render: WorldRender::new(),
            cursor_line_vertices: vertex_buffer,
            cursor_line_indices: index_buffer,
            yaw: 0.,
            pitch: 0.,
            block_target: None,
        }
    }

    pub fn run(&mut self) {
        while self.drain_events() {
            self.update_block_target();
            self.write_cursor();
            let pos = BlockPos([self.camera.position[0] as i32, self.camera.position[1] as i32, self.camera.position[2] as i32]);
            self.world_render.update(&pos, &self.world.read(), &self.display);
            self.render();
        }
    }

    fn write_cursor(&mut self) {
        let look_at = self.block_target.clone().unwrap_or(ray::BlockIntersection {
            block: BlockPos([1_000_000, 1_000_000, 1_000_000]),
            face: Direction::PosX,
        });
        let look_at_base = [look_at.block[0] as f32, look_at.block[1] as f32, look_at.block[2] as f32];
        let look_at_corners = CUBE_FACES[look_at.face as usize];
        let center = vec3_add(self.camera.position, vec3_scale(self.camera.forward, 10.));
        self.cursor_line_vertices.write(&[
            //cursor
            LineVertex { pos: center, color: [1., 0., 0.] },
            LineVertex { pos: vec3_add(center, [1., 0., 0.]), color: [1., 0., 0.] },
            LineVertex { pos: center, color: [0., 1., 0.] },
            LineVertex { pos: vec3_add(center, [0., 1., 0.]), color: [0., 1., 0.] },
            LineVertex { pos: center, color: [0., 0., 1.] },
            LineVertex { pos: vec3_add(center, [0., 0., 1.]), color: [0., 0., 1.] },
            //look at cross
            LineVertex { pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[0]]), color: [1., 1., 0.] },
            LineVertex { pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[2]]), color: [1., 1., 0.] },
            LineVertex { pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[1]]), color: [1., 1., 0.] },
            LineVertex { pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[3]]), color: [1., 1., 0.] },
        ]);
    }

    fn drain_events(&mut self) -> bool {
        let mut cam_changed = false;
        for ev in self.display.poll_events() {
            use vecmath::{vec3_add, vec3_scale, vec3_sub};
            match ev {
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(glutin::VirtualKeyCode::Z)) => {
                    print!("pos: {:?}, dir: {:?}, look_at: {:?}", self.camera.position, self.camera.forward, self.block_target);
                    if let Some((target,direction)) = self.block_target.clone().map(|t| (t.block,t.face)) {
                        let print_block=target.facing(direction);
                        let env_data = self.world.env_data();
                        let world = self.world.read();
                        print!(" ({:?})\nid: {:?}\natural light: {:?}, artificial light: {:?}\n",
                                 print_block,
                                 world.get_block(&target).unwrap(),
                                 world.natural_light(&print_block).unwrap(),
                                 world.artificial_light(&print_block).unwrap(),
                        );
                        let (x, z) = (target[0], target[2]);
                        println!("temperature: {}, moisture: {}, base elevation: {}",
                               env_data.temperature(x, z),
                               env_data.moisture(x, z),
                               env_data.base_elevation(x, z)
                        );
                    } else {
                        println!()
                    }
                }
                glutin::Event::Closed => return false,
                glutin::Event::MouseMoved(x, y) => {
                    if let Ok((x, y)) = window_util::read_mouse_delta(&self.display, (x, y)) {
                        use std::f32::consts::PI;
                        self.yaw += x as f32 / 300.;
                        self.pitch = (self.pitch - y as f32 / 300.).min(0.5 * PI).max(-0.5 * PI);
                        self.camera.set_yaw_pitch(self.yaw, self.pitch);
                    }
                    cam_changed = true;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(glutin::VirtualKeyCode::W)) => {
                    self.camera.position = vec3_add(self.camera.position, vec3_scale(self.camera.forward, 0.5));
                    cam_changed = true;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(glutin::VirtualKeyCode::S)) => {
                    self.camera.position = vec3_sub(self.camera.position, vec3_scale(self.camera.forward, 0.5));
                    cam_changed = true;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(glutin::VirtualKeyCode::D)) => {
                    self.camera.position = vec3_add(self.camera.position, vec3_scale(self.camera.right, 0.5));
                    cam_changed = true;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(glutin::VirtualKeyCode::A)) => {
                    self.camera.position = vec3_sub(self.camera.position, vec3_scale(self.camera.right, 0.5));
                    cam_changed = true;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(glutin::VirtualKeyCode::E)) => {
                    self.camera.position = vec3_add(self.camera.position, vec3_scale(self.camera.up, 0.5));
                    cam_changed = true;
                },
                glutin::Event::KeyboardInput(glutin::ElementState::Pressed, _, Some(glutin::VirtualKeyCode::Q)) => {
                    self.camera.position = vec3_sub(self.camera.position, vec3_scale(self.camera.up, 0.5));
                    cam_changed = true;
                },
                glutin::Event::MouseInput(state, button) => {
                    let button_id = match button {
                        glutin::MouseButton::Left => 0,
                        glutin::MouseButton::Right => 1,
                        _ => continue,
                    };
                    match state {
                        glutin::ElementState::Pressed => {
                            self.event_sender.send(Message::MousePressed { button: button_id }).unwrap();
                        },
                        glutin::ElementState::Released => {
                            self.event_sender.send(Message::MouseReleased { button: button_id }).unwrap();
                        },
                    }
                },
                _ => ()
            }
        }
        if cam_changed {
            self.event_sender.send(Message::CamChanged { pos: self.camera.position, direction: self.camera.forward }).unwrap();
        }
        true
    }

    fn update_block_target(&mut self) {
        let new_block_target = self.world.read().block_ray_trace(self.camera.position, self.camera.forward, 20.);
        if new_block_target != self.block_target {
            self.block_target = new_block_target.clone();
            self.event_sender.send(Message::BlockTargetChanged { target: new_block_target }).unwrap();
        }
    }

    fn render(&mut self) {
        let mut target = self.display.draw();
        target.clear_color_and_depth((0.5, 0.5, 0.5, 1.), 1.0);
        {
            let perspective = {
                let f = (0.5 as f32).tan();
                let aspect_ratio = 9. / 16.;
                let zfar = 1000.;
                let znear = 0.01;
                [
                    [f * aspect_ratio, 0.0, 0.0, 0.0],
                    [0.0, f, 0.0, 0.0],
                    [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                    [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
                ]
            };
            let matrix = col_mat4_mul(perspective, self.camera.orthogonal());
            let sampler = self.textures.sampled().wrap_function(SamplerWrapFunction::Repeat);
            self.world_render.draw(&mut target, matrix, sampler, &self.quad_shader).unwrap();
            target.draw(&self.cursor_line_vertices, &self.cursor_line_indices, &self.line_shader, &uniform! {transform:matrix}, &Default::default()).unwrap();
        }
        target.finish().unwrap();
    }
}
