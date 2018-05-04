use cam::Camera;
use world::timekeeper::TickId;
use player::{PLAYER_SIZE, Player};
use std::sync::Arc;
use std::f64::consts::PI;
use super::position_interpolator::PositionInterpolator;

const PLAYER_VIEW_Y: f64 = 1.6;
const PLAYER_CAMERA_OFFSET: [f64; 3] = [PLAYER_SIZE[0] / 2., PLAYER_VIEW_Y, PLAYER_SIZE[2] / 2.];

pub struct PlayerController {
    camera: Camera<f64>,
    pos: PositionInterpolator,
    yaw: f64,
    pitch: f64,
    player: Arc<Player>,
}

impl PlayerController {
    pub fn new(player: Arc<Player>, pos: PositionInterpolator) -> Self {
        PlayerController {
            camera: Camera::new([0.0; 3]),
            pos,
            yaw: 0.,
            pitch: 0.,
            player,
        }
    }

    pub fn ignores_physics(&self) -> bool {
        self.player.ignores_physics()
    }

    pub fn set_ignores_physics(&mut self, b: bool) {
        self.player.set_ignores_physics(b)
    }

    pub fn jump(&mut self) {
        self.player.jump()
    }

    pub fn get_player(&self) -> &Arc<Player> {
        &self.player
    }

    pub fn position(&mut self) -> [f64; 3] {
        self.pos.most_recent()
    }

    pub fn change_look(&mut self, d_yaw: f64, d_pitch: f64) {
        self.yaw = ((self.yaw + d_yaw) / 2. / PI).fract() * 2. * PI;
        self.pitch = (self.pitch - d_pitch).min(0.5 * PI).max(-0.5 * PI);
        self.camera.set_yaw_pitch(self.yaw, self.pitch);
    }

    pub fn set_movement(&mut self, m: [f64; 3]) {
        use vecmath::*;
        let v = if self.player.ignores_physics() {
            let v = [0.; 3];
            let v = vec3_add(v, vec3_scale(self.camera.forward, m[0] * 10.));
            let v = vec3_add(v, vec3_scale(self.camera.up, m[1] * 10.));
            let v = vec3_add(v, vec3_scale(self.camera.right, m[2] * 10.));
            v
        } else {
            let (sin, cos) = self.yaw.sin_cos();
            let (sin, cos) = (sin as f64, cos as f64);
            let v = [
                sin * m[0] + cos * m[2],
                0.,
                cos * m[0] - sin * m[2],
            ];
            v
        };
        self.player.set_movement_control(v)
    }


    pub fn look_direction(&self) -> [f64; 3] {
        self.camera.forward
    }

    pub fn sub_tick_camera(&mut self, tick: TickId, sub_tick_time: f32) -> Camera<f64> {
        use vecmath::*;
        let pos = self.pos.pos(tick, sub_tick_time);
        Camera {
            position: vec3_add(pos, PLAYER_CAMERA_OFFSET),
            ..self.camera
        }
    }
}
