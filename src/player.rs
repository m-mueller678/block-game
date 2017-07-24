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

    pub fn set_movement(&mut self,mut m:[f64;3]){
        use vecmath::vec3_normalized;
        m[1]=0.;
        let movement=vec3_normalized(m);
        let (sin,cos)=self.yaw.sin_cos();
        let (sin,cos)=(sin as f64,cos as f64);
        let rot_movement=[
            cos*movement[0]-sin*movement[2],
            self.physics.v()[1],
            sin*movement[0]+cos*movement[2],
        ];
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