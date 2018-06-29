use world::{World, timekeeper::TickId, LoadGuard, BlockPos, ChunkPos};
use physics::Object as PhysObject;
use block::BlockId;
use geometry::ray::BlockIntersection;
use item::{SlotStorage, Slot};
use std::sync::Mutex;
use ui::{PositionUpdateSender, Message};
use std::sync::mpsc::{Receiver, TryRecvError};

struct PlayerPhysics {
    object: PhysObject,
    ignores_physics: bool,
    movement_control: [f64; 3],
}

struct PlayerInterface {
    chunk_load_guard: LoadGuard,
    mouse_pressed_since: [Option<TickId>; 2],
    block_target: Option<BlockIntersection>,
    rec: Receiver<Message>,
}

pub struct Player {
    physics: Mutex<PlayerPhysics>,
    inventory: SlotStorage,
    held_item: Slot,
    position_update: PositionUpdateSender,
    interface: Mutex<PlayerInterface>,
}

pub const PLAYER_SIZE: [f64; 3] = [0.6, 1.8, 0.6];
const PLAYER_MAX_SPEED: f64 = 4.0;

impl Player {
    pub fn new(
        position_update: PositionUpdateSender,
        world: &World,
        ui_rec: Receiver<Message>,
    ) -> Self {
        Player {
            physics: Mutex::new(PlayerPhysics {
                object: PhysObject::new(PLAYER_SIZE),
                ignores_physics: false,
                movement_control: [0.0; 3],
            }),
            interface: Mutex::new(PlayerInterface {
                chunk_load_guard: world.load_cube(ChunkPos([0; 3]), 2),
                block_target: None,
                rec: ui_rec,
                mouse_pressed_since: [None; 2],
            }),
            inventory: SlotStorage::new(40),
            held_item: Slot::new(),
            position_update,
        }
    }

    pub fn set_ignores_physics(&self, b: bool) {
        let mut physics = self.physics.lock().unwrap();
        physics.ignores_physics = b;
        if !b {
            Self::normalize_speed(&mut physics.movement_control)
        }
    }

    fn normalize_speed(v: &mut [f64; 3]) {
        use vecmath::vec3_inv_len;
        let inv_len = vec3_inv_len(*v);
        if inv_len.is_finite() {
            let f = inv_len * PLAYER_MAX_SPEED;
            for x in v {
                *x *= f;
            }
        } else {
            *v = [0.0; 3];
        }
    }

    pub fn ignores_physics(&self) -> bool {
        self.physics.lock().unwrap().ignores_physics
    }

    pub fn set_movement_control(&self, mut m: [f64; 3]) {
        let mut physics = self.physics.lock().unwrap();
        if !physics.ignores_physics {
            Self::normalize_speed(&mut m);
        }
        physics.movement_control = m;
        if physics.ignores_physics {
            physics.object.set_v(m);
        }
    }

    pub fn tick(&self, tick: TickId, world: &World) {
        let player_pos = self.physics_tick(tick, world);
        self.interface_tick(tick, world, player_pos);
    }

    fn interface_tick(&self, tick: TickId, world: &World, player_pos: BlockPos) {
        use glium::glutin::{MouseButton, ElementState};

        let chunk_pos = player_pos.pos_in_chunk().0;
        let mut interface = self.interface.lock().unwrap();
        if chunk_pos != interface.chunk_load_guard.center() {
            interface.chunk_load_guard = world.load_cube(chunk_pos, 2);
        }
        loop {
            match interface.rec.try_recv() {
                Ok(Message::BlockTargetChanged { target }) => {
                    for p in &mut interface.mouse_pressed_since {
                        *p = p.map(|_| tick);
                    }
                    interface.block_target = target;
                }
                Ok(Message::MouseInput {
                       state: ElementState::Pressed,
                       button,
                   }) => {
                    interface.mouse_pressed_since[match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        _ => continue,
                    }] = Some(tick);
                    if button == MouseButton::Right {
                        if let Some(ref block_target) = interface.block_target {
                            world
                                .set_block(
                                    block_target.block.facing(block_target.face),
                                    world.game_data().blocks().by_name("debug_light").unwrap(),
                                )
                                .is_ok();
                        }
                    }
                }
                Ok(Message::MouseInput {
                       state: ElementState::Released,
                       button,
                   }) => {
                    interface.mouse_pressed_since[match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        _ => continue,
                    }] = None;
                }
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }

        if let Some(block_target) = interface.block_target.clone() {
            if let Some(pressed_since) = interface.mouse_pressed_since[0] {
                if tick.ticks_since(pressed_since) >= 10 {
                    world
                        .set_block(block_target.block, BlockId::empty())
                        .is_ok();
                }
            } else if let Some(pressed_since) = interface.mouse_pressed_since[1] {
                if tick.ticks_since(pressed_since) >= 10 {
                    world
                        .set_block(
                            block_target.block.facing(block_target.face),
                            world.game_data().blocks().by_name("debug_light").unwrap(),
                        )
                        .is_ok();
                }
            }
        }
    }

    pub fn jump(&self) {
        let mut physics = self.physics.lock().unwrap();
        if physics.object.on_ground() {
            let mut v = physics.object.v();
            v[1] = 4.8;
            physics.object.set_v(v);
        }
    }

    pub fn inventory(&self) -> &SlotStorage {
        &self.inventory
    }

    pub fn held_item(&self) -> &Slot {
        &self.held_item
    }

    fn physics_tick(&self, tick: TickId, world: &World) -> BlockPos {
        use vecmath::{vec3_add, vec3_scale};

        let position = {
            let mut physics = self.physics.lock().unwrap();
            if physics.ignores_physics {
                physics.object.tick(None, false);
                physics.object.position()
            } else {
                if physics.object.on_ground() {
                    let mc = physics.movement_control;
                    physics.object.set_v(mc);
                } else {
                    let new_v = vec3_add(physics.object.v(), vec3_scale(physics.movement_control, 1. / 256.));
                    physics.object.set_v(new_v)
                }
                physics.object.tick(Some(world), true);
                physics.object.position()
            }
        };
        self.position_update.send(position, tick);
        BlockPos([
            position[0].floor() as i32,
            position[1].floor() as i32,
            position[2].floor() as i32,
        ])
    }
}
