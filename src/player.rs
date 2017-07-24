use std::f32::consts::PI;
use cam::Camera;
use world::WorldReadGuard;
use physics::Object as PhysObject;

pub struct Player{
    physics:PhysObject,
    camera:Camera,
    yaw:f32,
    pitch:f32,
}

impl Player{
    pub fn new()->Self{
        Player{
            physics:PhysObject::new([0.6,1.8,0.6]),
            camera:Camera::new([0.;3]),
            yaw:0.,
            pitch:0.,
        }
    }

    pub fn change_look(&mut self,d_yaw:f32,d_pitch:f32){
        self.yaw += d_yaw;
        self.pitch = (self.pitch - d_pitch).min(0.5 * PI).max(-0.5 * PI);
        self.camera.set_yaw_pitch(self.yaw, self.pitch);
    }

    pub fn position(&self)->[f64;3]{
        self.physics.position()
    }

    pub fn tick(&mut self,world:&WorldReadGuard){
        self.physics.tick(world);
        let p=self.physics.position();
        self.camera.position=[p[0]as f32,p[1]as f32,p[2]as f32];
    }

    pub fn set_movement(&mut self,m:[f64;3]){
        let len=(m[0].powi(2)+m[1].powi(2)).sqrt();
        let rot_movement=if len<1e-6{
            [
                0.,
                self.physics.v()[1],
                0.,
            ]
        }else{
            let (sin,cos)=self.yaw.sin_cos();
            let (sin,cos)=(sin as f64,cos as f64);
            [
                (cos*m[0]-sin*m[2])/len,
                self.physics.v()[1],
                (sin*m[0]+cos*m[2])/len,
            ]
        };
        self.set_v(rot_movement);
    }

    pub fn set_v(&mut self,v:[f64;3]){
        self.physics.set_v(v);
    }

    pub fn look_direction(&self)->[f32;3]{
        self.camera.forward
    }

    pub fn camera(&self)->&Camera{
        &self.camera
    }
}