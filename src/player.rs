use std::f64::consts::PI;
use cam::Camera;
use world::WorldReadGuard;
use physics::Object as PhysObject;
use item::{SlotStorage, Slot};
use std::sync::Mutex;

struct PlayerCamera {
    camera: Camera<f64>,
    yaw: f64,
    pitch: f64,
}

impl Clone for PlayerCamera {
    fn clone(&self) -> Self {
        PlayerCamera {
            camera: Camera {
                ..self.camera
            },
            ..*self
        }
    }
}

struct PlayerPhysics {
    object: PhysObject,
    ignores_physics: bool,
}

pub struct Player {
    physics: Mutex<PlayerPhysics>,
    camera: Mutex<PlayerCamera>,
    inventory: SlotStorage,
    held_item: Slot,
}

pub const PLAYER_SIZE: [f64; 3] = [0.6, 1.8, 0.6];
const PLAYER_VIEW_Y: f64 = 1.6;
const PLAYER_CAMERA_OFFSET: [f64; 3] = [
    PLAYER_SIZE[0] / 2.,
    PLAYER_VIEW_Y,
    PLAYER_SIZE[2] / 2.,
];

impl Player {
    pub fn new() -> Self {
        Player {
            physics: Mutex::new(PlayerPhysics {
                object: PhysObject::new(PLAYER_SIZE),
                ignores_physics: false,
            }),
            camera: Mutex::new(PlayerCamera {
                camera: Camera::new(PLAYER_CAMERA_OFFSET),
                yaw: 0.,
                pitch: 0.,
            }),
            inventory: SlotStorage::new(40),
            held_item: Slot::new(),
        }
    }

    pub fn set_ignores_physics(&self, b: bool) {
        self.physics.lock().unwrap().ignores_physics = b;
    }

    pub fn ignores_physics(&self) -> bool {
        self.physics.lock().unwrap().ignores_physics
    }

    pub fn change_look(&self, d_yaw: f64, d_pitch: f64) {
        let PlayerCamera { ref mut camera, ref mut yaw, ref mut pitch } = *self.camera.lock().unwrap();
        *yaw = ((*yaw + d_yaw) / 2. / PI).fract() * 2. * PI;
        *pitch = (*pitch - d_pitch).min(0.5 * PI).max(-0.5 * PI);
        camera.set_yaw_pitch(*yaw, *pitch);
    }

    pub fn position(&self) -> [f64; 3] {
        self.physics.lock().unwrap().object.position()
    }

    pub fn tick(&self, world: &WorldReadGuard) {
        use vecmath::vec3_add;
        let position = {
            let mut physics = self.physics.lock().unwrap();
            if physics.ignores_physics {
                physics.object.tick(None, false);
            } else {
                physics.object.tick(Some(world), true);
            }
            physics.object.position()
        };
        self.camera.lock().unwrap().camera.position = vec3_add(position, PLAYER_CAMERA_OFFSET);
    }

    pub fn set_movement(&self, m: [f64; 3]) {
        use vecmath::*;
        let camera: PlayerCamera = self.camera.lock().unwrap().clone();
        let mut physics = self.physics.lock().unwrap();
        if physics.ignores_physics {
            let v = [0.; 3];
            let v = vec3_add(v, vec3_scale(camera.camera.forward, m[0] * 10.));
            let v = vec3_add(v, vec3_scale(camera.camera.up, m[1] * 10.));
            let v = vec3_add(v, vec3_scale(camera.camera.right, m[2] * 10.));
            physics.object.set_v(v);
        } else {
            let len = (m[0].powi(2) + m[2].powi(2)).sqrt();
            let rot_movement = if len < 1e-6 {
                [
                    0.,
                    physics.object.v()[1],
                    0.,
                ]
            } else {
                let (sin, cos) = camera.yaw.sin_cos();
                let (sin, cos) = (sin as f64, cos as f64);
                [
                    (sin * m[0] + cos * m[2]) * 4. / len,
                    physics.object.v()[1],
                    (cos * m[0] - sin * m[2]) * 4. / len,
                ]
            };
            physics.object.set_v(rot_movement);
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

    pub fn look_direction(&self) -> [f64; 3] {
        self.camera.lock().unwrap().camera.forward
    }

    pub fn sub_tick_camera(&self, sub_tick_time: f32) -> Camera<f64> {
        use vecmath::*;
        let pos = {
            let PlayerPhysics { ref mut object, .. } = *self.physics.lock().unwrap();
            let dif = vec3_sub(object.position(), object.previous_tick_position());
            let scaled = vec3_scale(dif, f64::from(sub_tick_time));
            vec3_add(vec3_add(object.previous_tick_position(), PLAYER_CAMERA_OFFSET), scaled)
        };
        Camera {
            position: pos,
            ..self.camera.lock().unwrap().camera
        }
    }

    pub fn inventory(&self) -> &SlotStorage {
        &self.inventory
    }

    pub fn held_item(&self) -> &Slot {
        &self.held_item
    }
}