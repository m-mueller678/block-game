use std::f64::consts::PI;
use cam::Camera;
use world::WorldReadGuard;
use physics::Object as PhysObject;

pub struct Player {
    physics: PhysObject,
    camera: Camera<f64>,
    yaw: f64,
    pitch: f64,
    ignores_physics: bool,
}

impl Player {
    pub fn new() -> Self {
        Player {
            physics: PhysObject::new([0.6, 1.8, 0.6]),
            camera: Camera::new([0.; 3]),
            yaw: 0.,
            pitch: 0.,
            ignores_physics: false,
        }
    }

    pub fn set_ignores_physics(&mut self, b: bool) {
        self.ignores_physics = b;
    }

    pub fn ignores_physics(&self) -> bool {
        self.ignores_physics
    }

    pub fn change_look(&mut self, d_yaw: f64, d_pitch: f64) {
        self.yaw += d_yaw;
        let yaw1 = self.yaw / 2. / PI;
        let yaw1 = yaw1 - yaw1.floor();
        self.yaw = yaw1 * 2. * PI;
        self.yaw = (self.yaw / 2. / PI).fract() * 2. * PI;
        self.pitch = (self.pitch - d_pitch).min(0.5 * PI).max(-0.5 * PI);
        self.camera.set_yaw_pitch(self.yaw, self.pitch);
    }

    pub fn position(&self) -> [f64; 3] {
        self.physics.position()
    }

    pub fn tick(&mut self, world: &WorldReadGuard) {
        if self.ignores_physics {
            self.physics.tick(None, false);
        } else {
            self.physics.tick(Some(world), true);
        }
        self.camera.position = self.physics.position();
    }

    pub fn set_movement(&mut self, m: [f64; 3]) {
        use vecmath::*;
        if self.ignores_physics {
            let v = [0.; 3];
            let v = vec3_add(v, vec3_scale(self.camera.forward, m[0] * 10.));
            let v = vec3_add(v, vec3_scale(self.camera.up, m[1] * 10.));
            let v = vec3_add(v, vec3_scale(self.camera.right, m[2] * 10.));
            self.set_v(v);
        } else {
            let len = (m[0].powi(2) + m[2].powi(2)).sqrt();
            let rot_movement = if len < 1e-6 {
                [
                    0.,
                    self.physics.v()[1],
                    0.,
                ]
            } else {
                let (sin, cos) = self.yaw.sin_cos();
                let (sin, cos) = (sin as f64, cos as f64);
                [
                    (sin * m[0] + cos * m[2]) * 4. / len,
                    self.physics.v()[1],
                    (cos * m[0] - sin * m[2]) * 4. / len,
                ]
            };
            self.set_v(rot_movement);
        }
    }

    pub fn set_v(&mut self, v: [f64; 3]) {
        self.physics.set_v(v);
    }

    pub fn look_direction(&self) -> [f64; 3] {
        self.camera.forward
    }

    pub fn camera(&self) -> &Camera<f64> {
        &self.camera
    }
}