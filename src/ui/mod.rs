use glium::texture::CompressedSrgbTexture2dArray;
use glium::glutin::*;
use glium::backend::glutin::Display;
use glium::*;
use glium::uniforms::SamplerWrapFunction;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use graphics::*;
use world::{BlockPos, World};
use geometry::*;
use vecmath::{vec3_add, vec3_scale, col_mat4_mul,mat4_cast};
use window_util;
use player::Player;

mod keyboard_state;

use self::keyboard_state::KeyboardState;

pub enum Message {
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    BlockTargetChanged { target: Option<ray::BlockIntersection> },
}

fn to_f32(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

pub struct Ui {
    display: Display,
    shader: Shader,
    event_sender: Sender<Message>,
    textures: CompressedSrgbTexture2dArray,
    world: Arc<World>,
    world_render: WorldRender,
    cursor_line_vertices: VertexBuffer<LineVertex>,
    cursor_line_indices: IndexBuffer<u32>,
    block_target: Option<ray::BlockIntersection>,
    overlays: Vec<(BlockOverlay, String)>,
    current_overlay: usize,
    player: Arc<Mutex<Player>>,
    key_state: KeyboardState,
    running: bool,
}

impl Ui {
    pub fn new(
        display: Display,
        shader: Shader,
        event_sender: Sender<Message>,
        textures: CompressedSrgbTexture2dArray,
        world: Arc<World>,
        player: Arc<Mutex<Player>>,
    ) -> Self {
        let index_buffer = IndexBuffer::<u32>::new
            (&display, index::PrimitiveType::LinesList, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
        let vertex_buffer = VertexBuffer::new(&display, &
            [LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] }; 10]).unwrap();
        let mut ret = Ui {
            display: display,
            shader: shader,
            event_sender: event_sender,
            textures: textures,
            world: world,
            world_render: WorldRender::new(),
            cursor_line_vertices: vertex_buffer,
            cursor_line_indices: index_buffer,
            block_target: None,
            overlays: Vec::new(),
            current_overlay: 0,
            player: player,
            key_state: KeyboardState::new(),
            running: true,
        };
        ret.load_overlays();
        ret
    }

    pub fn run(&mut self,events:&mut EventsLoop) {
        loop {
            self.drain_events(events);
            if !self.running {
                break;
            }
            let (camera_position, forward) = {
                let player = self.player.lock().unwrap();
                (player.camera().position, player.look_direction())
            };
            self.update_block_target(camera_position, forward);
            self.write_cursor(camera_position,forward);
            let pos = BlockPos([
                camera_position[0].floor() as i32,
                camera_position[1].floor() as i32,
                camera_position[2].floor() as i32
            ]);
            self.world_render.update(&pos, &self.world.read(), &self.display);
            self.render();
        }
    }

    fn write_cursor(&mut self,position:[f64;3],player_forward:[f64;3]) {
        let look_at = self.block_target.clone().unwrap_or(ray::BlockIntersection {
            block: BlockPos([1_000_000, 1_000_000, 1_000_000]),
            face: Direction::PosX,
        });
        let look_at_base = [look_at.block[0] as f32, look_at.block[1] as f32, look_at.block[2] as f32];
        let look_at_corners = CUBE_FACES[look_at.face as usize];
        let center = to_f32(vec3_add(position, vec3_scale(player_forward, 10.)));
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

    fn process_event(&mut self, evt: Event) {
        let id = self.display.gl_window().id();
        match evt {
            Event::WindowEvent { window_id, ref event } if window_id == id => {
                self.process_window_event(event)
            }
            _ => {}
        }
    }

    fn process_window_event(&mut self, evt: &WindowEvent) {
        match *evt {
            WindowEvent::KeyboardInput { input, .. } => {
                self.key_state.update(&input);
                self.process_keyboard_event(&input)
            }
            WindowEvent::Closed => { self.running = false }
            WindowEvent::MouseMoved { position: (x, y), .. } => {
                //TODO use raw input
                if let Ok((x, y)) = window_util::read_mouse_delta(&self.display, (x, y)) {
                    self.player.lock().unwrap().change_look(x / 300., y  / 300.);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.event_sender.send(Message::MouseInput {
                    button: button,
                    state: state,
                }).unwrap();
            }
            _ => {}
        }
    }

    fn process_keyboard_event(&mut self, key: &KeyboardInput) {
        if key.state != ElementState::Pressed {
            return;
        }
        match key.virtual_keycode {
            Some(VirtualKeyCode::Z) => {
                let player = self.player.lock().unwrap();
                print!("pos: {:?}, dir: {:?}, look_at: {:?}", player.position(), player.look_direction(), self.block_target);
                if let Some((target, direction)) = self.block_target.clone().map(|t| (t.block, t.face)) {
                    let facing_block = target.facing(direction);
                    let world_r = self.world.read();
                    println!(" ({:?})", facing_block);
                    println!("id: {:?}", world_r.get_block(&target).unwrap());
                    println!("natural light: {:?}, artificial light: {:?}",
                             world_r.natural_light(&facing_block).unwrap(),
                             world_r.artificial_light(&facing_block).unwrap()
                    );
                    println!("gen-biome: {}", self.world.biomes()[self.world.generator().biome_at(target[0], target[2])].name());
                } else {
                    println!()
                }
            }
            Some(VirtualKeyCode::O) => {
                self.current_overlay = (self.current_overlay + 1) % (self.overlays.len() + 1);
                println!("set overlay to: {:?}", self.overlays.get(self.current_overlay).map(|o| &o.1));
            }
            Some(VirtualKeyCode::G)=>{
                let mut player=self.player.lock().unwrap();
                let set_to=!player.ignores_physics();
                player.set_ignores_physics(set_to);
                println!("ignore physics set to: {}",set_to);
            }
            Some(VirtualKeyCode::Space)=>{
                self.player.lock().unwrap().jump();
            }
            _ => {}
        }
    }

    fn drain_events(&mut self,events:&mut EventsLoop) {
        events.poll_events(|e| self.process_event(e));
        let mut movement=[0.;3];
        if self.key_state.pressed(VirtualKeyCode::W) {movement[0]+=1.;}
        if self.key_state.pressed(VirtualKeyCode::S) {movement[0]-=1.;}
        if self.key_state.pressed(VirtualKeyCode::E) {movement[1]+=1.;}
        if self.key_state.pressed(VirtualKeyCode::Q) {movement[1]-=1.;}
        if self.key_state.pressed(VirtualKeyCode::D) {movement[2]+=1.;}
        if self.key_state.pressed(VirtualKeyCode::A) {movement[2]-=1.;}
        self.player.lock().unwrap().set_movement(movement);
    }

    fn update_block_target(&mut self, position: [f64; 3], forward: [f64; 3]) {
        //TODO make rays work with f64
        let new_block_target = self.world.read().block_ray_trace(to_f32(position), to_f32(forward), 100.);
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
                let zfar = 400.;
                let znear = 0.01;
                [
                    [f * aspect_ratio, 0.0, 0.0, 0.0],
                    [0.0, f, 0.0, 0.0],
                    [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                    [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
                ]
            };
            let matrix = col_mat4_mul(perspective, mat4_cast(self.player.lock().unwrap().camera().orthogonal()));
            let sampler = self.textures.sampled().wrap_function(SamplerWrapFunction::Repeat);
            self.world_render.draw(&mut target, matrix, sampler, &self.shader.quad).unwrap();
            if let Some(overlay) = self.overlays.get_mut(self.current_overlay) {
                overlay.0.draw(&mut target, &self.shader.overlay, matrix).unwrap();
            }
            target.draw(&self.cursor_line_vertices, &self.cursor_line_indices, &self.shader.line, &uniform! {transform:matrix}, &Default::default()).unwrap();
        }
        target.finish().unwrap();
    }

    fn load_overlays(&mut self) {
        self.overlays = vec![];
        self.current_overlay = self.overlays.len();
    }
}