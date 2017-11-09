use std::sync::mpsc::Sender;
use std::sync::Arc;use glium::glutin::*;
use glium::*;
use glium::uniforms::SamplerWrapFunction;
use vecmath::{vec3_add, vec3_scale, col_mat4_mul, mat4_cast};
use cam::Camera;
use window_util;
use graphics::*;
use geometry::*;
use player::Player;
use world::{BlockPos, World};
use module::GameData;
use super::{KeyboardState, Message};
pub use super::UiState;
use super::ui_core::UiCore;

fn to_f32(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

pub struct GameUi {
    event_sender: Sender<Message>,
    world: Arc<World>,
    world_render: WorldRender,
    cursor_line_vertices: VertexBuffer<LineVertex>,
    cursor_line_indices: IndexBuffer<u32>,
    block_target: Option<ray::BlockIntersection>,
    overlays: Vec<(BlockOverlay, String)>,
    current_overlay: usize,
    player: Arc<Player>,
    camera: Camera<f64>,
    game_data: GameData,
}

impl GameUi {
    pub fn new(
        event_sender: Sender<Message>,
        world: Arc<World>,
        player: Arc<Player>,
        core: &UiCore,
    ) -> Self {
        let index_buffer = IndexBuffer::<u32>::new(
            &core.display,
            index::PrimitiveType::LinesList,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        ).unwrap();
        let vertex_buffer = VertexBuffer::new(
            &core.display,
            &[LineVertex {
                pos: [0., 0., 0.],
                color: [1., 1., 0.],
            }; 10],
        ).unwrap();
        let camera = player.sub_tick_camera(0.);
        let mut ret = GameUi {
            event_sender: event_sender,
            game_data: Arc::clone(world.game_data()),
            world_render: WorldRender::new(Arc::clone(&world)),
            world: world,
            cursor_line_vertices: vertex_buffer,
            cursor_line_indices: index_buffer,
            block_target: None,
            overlays: Vec::new(),
            current_overlay: 0,
            player: player,
            camera: camera,
        };
        ret.load_overlays();
        ret
    }

    pub fn update_and_render(&mut self, ui_core: &UiCore, state: &UiState, target: &mut Frame) {
        let pos = BlockPos(
            [
                self.camera.position[0].floor() as i32,
                self.camera.position[1].floor() as i32,
                self.camera.position[2].floor() as i32,
            ],
        );
        self.world_render.update(
            pos,
            &ui_core.display,
        );
        {
            let time = self.world.time().sub_tick_time();
            if let UiState::InGame = *state {
                let movement = Self::read_movement(&ui_core.key_state);
                self.player.set_movement(movement);
            }
            self.camera = self.player.sub_tick_camera(time);
        }
        self.update_block_target();
        self.write_cursor();
        self.render(ui_core, target);
    }

    fn read_movement(kb: &KeyboardState) -> [f64; 3] {
        let mut movement = [0.; 3];
        if kb.pressed(VirtualKeyCode::W) {
            movement[0] += 1.;
        }
        if kb.pressed(VirtualKeyCode::S) {
            movement[0] -= 1.;
        }
        if kb.pressed(VirtualKeyCode::E) {
            movement[1] += 1.;
        }
        if kb.pressed(VirtualKeyCode::Q) {
            movement[1] -= 1.;
        }
        if kb.pressed(VirtualKeyCode::D) {
            movement[2] += 1.;
        }
        if kb.pressed(VirtualKeyCode::A) {
            movement[2] -= 1.;
        }
        movement
    }

    fn write_cursor(&mut self) {
        let look_at = self.block_target.clone().unwrap_or(ray::BlockIntersection {
            block: BlockPos([1_000_000, 1_000_000, 1_000_000]),
            face: Direction::PosX,
        });
        let look_at_base = [
            look_at.block[0] as f32,
            look_at.block[1] as f32,
            look_at.block[2] as f32,
        ];
        let look_at_corners = CUBE_FACES[look_at.face as usize];
        let center = to_f32(vec3_add(
            self.camera.position,
            vec3_scale(self.camera.forward, 10.),
        ));
        self.cursor_line_vertices.write(
            &[
                //cursor
                LineVertex {
                    pos: center,
                    color: [1., 0., 0.],
                },
                LineVertex {
                    pos: vec3_add(center, [1., 0., 0.]),
                    color: [1., 0., 0.],
                },
                LineVertex {
                    pos: center,
                    color: [0., 1., 0.],
                },
                LineVertex {
                    pos: vec3_add(center, [0., 1., 0.]),
                    color: [0., 1., 0.],
                },
                LineVertex {
                    pos: center,
                    color: [0., 0., 1.],
                },
                LineVertex {
                    pos: vec3_add(center, [0., 0., 1.]),
                    color: [0., 0., 1.],
                },
                //look at cross
                LineVertex {
                    pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[0]]),
                    color: [1., 1., 0.],
                },
                LineVertex {
                    pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[2]]),
                    color: [1., 1., 0.],
                },
                LineVertex {
                    pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[1]]),
                    color: [1., 1., 0.],
                },
                LineVertex {
                    pos: vec3_add(look_at_base, CORNER_OFFSET[look_at_corners[3]]),
                    color: [1., 1., 0.],
                },
            ],
        );
    }

    pub fn process_window_event(
        &mut self,
        evt: &WindowEvent,
        ui_core: &mut UiCore,
        state: &mut UiState,
    ) {
        match *evt {
            WindowEvent::KeyboardInput { input, .. } => self.process_keyboard_event(&input, state),
            WindowEvent::MouseMoved { position: (x, y), .. } => {
                //TODO use raw input
                if let Ok((x, y)) = window_util::read_mouse_delta(&ui_core.display, (x, y)) {
                    self.player.change_look(x / 300., y / 300.);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.event_sender
                    .send(Message::MouseInput {
                        button: button,
                        state: state,
                    })
                    .unwrap();
            }
            _ => {}
        }
    }

    fn process_keyboard_event(&mut self, key: &KeyboardInput, state: &mut UiState) {
        if key.state != ElementState::Pressed {
            return;
        }
        match key.virtual_keycode {
            Some(VirtualKeyCode::Z) => {
                print!(
                    "pos: {:?}, dir: {:?}, look_at: {:?}",
                    self.player.position(),
                    self.player.look_direction(),
                    self.block_target
                );
                if let Some((target, direction)) =
                self.block_target.clone().map(|t| (t.block, t.face))
                    {
                        let facing_block = target.facing(direction);
                        let world_r = self.world.read();
                        println!(" ({:?})", facing_block);
                        if let (Some(id), Some(nl), Some(al)) = (
                            world_r.get_block(target),
                            world_r.natural_light(facing_block),
                            world_r.artificial_light(facing_block)
                        ) {
                            println!("id: {:?}\nnatural light: {:?}, artificial light: {:?}", id, nl, al);
                        }
                        println!(
                            "gen-biome: {}",
                            self.world.game_data().biomes()[self.world.game_data().generator().biome_at(
                                target[0],
                                target[2],
                            )].name()
                        );
                    } else {
                    println!()
                }
            }
            Some(VirtualKeyCode::O) => {
                self.current_overlay = (self.current_overlay + 1) % (self.overlays.len() + 1);
                println!(
                    "set overlay to: {:?}",
                    self.overlays.get(self.current_overlay).map(|o| &o.1)
                );
            }
            Some(VirtualKeyCode::G) => {
                let set_to = !self.player.ignores_physics();
                self.player.set_ignores_physics(set_to);
                println!("ignore physics set to: {}", set_to);
            }
            Some(VirtualKeyCode::Space) => {
                self.player.jump();
            }
            Some(VirtualKeyCode::I) => {
                use super::menu::{PlayerInventory, MenuLayerController};
                self.player.set_movement([0.; 3]);
                *state = UiState::Menu(Box::new(MenuLayerController::new(vec![
                    Box::new(PlayerInventory::new(
                        Arc::clone(&self.game_data),
                        Arc::clone(&self.player),
                    )),
                ])));
            }
            _ => {}
        }
    }

    fn update_block_target(&mut self) {
        let new_block_target = self.world.read().block_ray_trace(
            to_f32(self.camera.position),
            to_f32(self.camera.forward),
            100.,
        );
        if new_block_target != self.block_target {
            self.block_target = new_block_target.clone();
            self.event_sender
                .send(Message::BlockTargetChanged { target: new_block_target })
                .unwrap();
        }
    }

    fn render(&mut self, ui_core: &UiCore, target: &mut Frame) {
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
            let matrix = col_mat4_mul(perspective, mat4_cast(self.camera.orthogonal()));
            let sampler = ui_core.textures.sampled().wrap_function(
                SamplerWrapFunction::Repeat,
            );
            self.world_render
                .draw(target, matrix, sampler, &ui_core.shader.quad)
                .unwrap();
            if let Some(overlay) = self.overlays.get_mut(self.current_overlay) {
                overlay
                    .0
                    .draw(target, &ui_core.shader.overlay, matrix)
                    .unwrap();
            }
            target
                .draw(
                    &self.cursor_line_vertices,
                    &self.cursor_line_indices,
                    &ui_core.shader.line,
                    &uniform! {transform:matrix},
                    &Default::default(),
                )
                .unwrap();
        }
    }

    fn load_overlays(&mut self) {
        self.overlays = vec![];
        self.current_overlay = self.overlays.len();
    }
}
