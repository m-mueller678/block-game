use world::{World, timekeeper::TickId};
use physics::Object as PhysObject;
use item::{SlotStorage, Slot};
use std::sync::Mutex;
use ui::PositionUpdateSender;

struct PlayerPhysics {
    object: PhysObject,
    ignores_physics: bool,
    movement_control: [f64; 3],
}

pub struct Player {
    physics: Mutex<PlayerPhysics>,
    inventory: SlotStorage,
    held_item: Slot,
    position_update: PositionUpdateSender,
}

pub const PLAYER_SIZE: [f64; 3] = [0.6, 1.8, 0.6];
const PLAYER_MAX_SPEED: f64 = 4.0;

impl Player {
    pub fn new(position_update: PositionUpdateSender) -> Self {
        Player {
            physics: Mutex::new(PlayerPhysics {
                object: PhysObject::new(PLAYER_SIZE),
                ignores_physics: false,
                movement_control: [0.0; 3],
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

    pub fn position(&self) -> [f64; 3] {
        self.physics.lock().unwrap().object.position()
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
}
