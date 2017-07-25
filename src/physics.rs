use world::{BlockPos,WorldReadGuard};
use TICK_TIME;

type V3 = [f64; 3];

pub struct Object {
    p: V3,
    v: V3,
    size: V3,
}

impl Object {
    pub fn new(size:[f64;3])->Self{
        Object{
            p:[0.;3],
            v:[0.;3],
            size:size
        }
    }

    pub fn set_v(&mut self,v:[f64;3]){
        self.v=v;
    }

    pub fn v(&self)->[f64;3]{
        self.v
    }

    pub fn tick(&mut self, collision_world:Option<&WorldReadGuard>,gravity:bool) {
        use vecmath::*;
        if let Some(world)=collision_world{
            for i in 0..3{
                self.move_axis(i,world);
            }
        }else{
            self.p=vec3_add(vec3_scale(self.v,TICK_TIME),self.p);
        }
        if gravity{
            self.v[1]-=TICK_TIME*10.;
        }
    }

    pub fn position(&self)->[f64;3]{
        self.p
    }

    fn move_axis(&mut self, axis: usize,world:&WorldReadGuard) {
        if self.v[axis].abs()<1e-6{
            return;
        }
        let move_positive=self.v[axis].is_sign_positive();
        let mut bounds = [[0.; 2]; 3];
        let mut move_front=0.;
        for i in 0..3 {
            if axis == i {
                move_front = if move_positive { self.p[i] + self.v[i] } else { self.p[i] };
                bounds[i][0] = move_front;
                bounds[i][1] = move_front + self.v[i] * TICK_TIME
            } else {
                bounds[i][0] = self.p[i];
                bounds[i][1] = self.p[i] + self.size[i];
            }
        }
        let range_x = [bounds[0][0].floor() as i32,bounds[0][1].ceil() as i32];
        let range_y = [bounds[1][0].floor() as i32,bounds[1][1].ceil() as i32];
        let range_z = [bounds[2][0].floor() as i32,bounds[2][1].ceil() as i32];
        let mut block_bounds=[
            [bounds[0][0]-range_x[0] as f64,bounds[0][1]-range_x[0] as f64],
            [0.;2],
            [0.;2],
        ];
        let mut min_collide_pos = TICK_TIME;
        for x in range_x[0]..range_x[1] {
            block_bounds[1][0]=bounds[1][0]-range_y[0] as f64;
            block_bounds[1][1]=bounds[1][1]-range_y[0] as f64;
            for y in range_y[0]..range_y[1] {
                block_bounds[2][0]=bounds[2][0]-range_z[0] as f64;
                block_bounds[2][1]=bounds[2][1]-range_z[0] as f64;
                for z in range_z[0]..range_z[1] {
                    let collide_pos=get_block_collision(block_bounds,
                                                        axis,
                                                        move_positive,
                                                        BlockPos([x,y,z]),
                                                        world
                    );
                    let collide_pos_abs=collide_pos+match axis{0=>x,1=>y,2=>z,_=>unreachable!()} as f64;
                    if (collide_pos_abs<min_collide_pos) ^ !move_positive{
                        min_collide_pos=collide_pos_abs;
                    }
                    block_bounds[2][0]-=1.;
                    block_bounds[2][1]-=1.;
                }
                block_bounds[1][0]-=1.;
                block_bounds[1][1]-=1.;
            }
            block_bounds[0][0]-=1.;
            block_bounds[0][1]-=1.;
        }
        let collide_time=(min_collide_pos-move_front)/self.v[axis];
        if collide_time<TICK_TIME{
            self.v[axis]=0.;
            self.p[axis]+=self.v[axis]*collide_time;
        }else{
            self.p[axis]+=self.v[axis]*TICK_TIME;
        }
    }
}

fn get_block_collision(block_bounds:[[f64;2];3],
                       move_axis:usize,
                       move_positive:bool,
                       p:BlockPos,
                       world:&WorldReadGuard)->f64{
    use block::BlockId;
    use std::f64;
    let empty=world.get_block(&p).map(|b|b==BlockId::empty()).unwrap_or(false);
    if empty{
        if move_positive{f64::INFINITY}else{f64::NEG_INFINITY}
    }else{
        if move_positive{0.}else{1.}
    }
}