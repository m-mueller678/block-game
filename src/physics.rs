use world::{BlockPos, WorldReadGuard};
use TICK_TIME;

type V3 = [f64; 3];

pub struct Object {
    p: V3,
    v: V3,
    previous_position:V3,
    size: V3,
    on_ground:bool
}

impl Object {
    pub fn new(size: [f64; 3]) -> Self {
        assert!(size.iter().all(|x| x.is_sign_positive()));
        Object {
            p: [0.; 3],
            v: [0.; 3],
            previous_position:[0.;3],
            size: size,
            on_ground:false,
        }
    }

    pub fn set_v(&mut self, v: [f64; 3]) {
        self.v = v;
        if v[1]>0.{
            self.on_ground=false;
        }
    }

    pub fn v(&self) -> [f64; 3] {
        self.v
    }

    pub fn on_ground(&self)->bool{
        self.on_ground
    }

    pub fn tick(&mut self, collision_world: Option<&WorldReadGuard>, gravity: bool) {
        self.previous_position=self.p;
        use vecmath::*;
        if let Some(world) = collision_world {
            for i in 0..3 {
                self.move_axis(i, world);
            }
        } else {
            self.p = vec3_add(vec3_scale(self.v, TICK_TIME), self.p);
            self.on_ground=false;
        }
        if gravity {
            self.v[1] -= TICK_TIME * 10.;
        }
    }

    pub fn position(&self) -> [f64; 3] {
        self.p
    }

    pub fn previous_tick_position(&self)->[f64;3]{
        self.previous_position
    }

    fn move_axis(&mut self, axis: usize, world: &WorldReadGuard) {
        use std::f64;
        if self.v[axis].abs() < 1e-6 {
            return;
        }
        let move_positive = self.v[axis].is_sign_positive();
        let mut bounds = [[0.; 2]; 3];
        let mut move_front = 0.;
        for i in 0..3 {
            if axis == i {
                move_front = if move_positive { self.p[i] + self.size[i] } else { self.p[i] };
                bounds[i][0] = move_front;
                bounds[i][1] = move_front + self.v[i] * TICK_TIME;
                if !move_positive {
                    bounds[i].swap(0, 1);
                }
            } else {
                bounds[i][0] = self.p[i];
                bounds[i][1] = self.p[i] + self.size[i];
            }
        }
        let range_x = [bounds[0][0].floor() as i32, bounds[0][1].ceil() as i32];
        let range_y = [bounds[1][0].floor() as i32, bounds[1][1].ceil() as i32];
        let range_z = [bounds[2][0].floor() as i32, bounds[2][1].ceil() as i32];
        let mut min_collide_pos = if move_positive{f64::INFINITY}else{f64::NEG_INFINITY};
        'find_collision: for x in range_x[0]..range_x[1] {
            for y in range_y[0]..range_y[1] {
                for z in range_z[0]..range_z[1] {
                    let mut block_bounds = bounds;
                    for b in &mut block_bounds[0] { *b -= x as f64; }
                    for b in &mut block_bounds[1] { *b -= y as f64; }
                    for b in &mut block_bounds[2] { *b -= z as f64; }
                    let collide_pos = get_block_collision(block_bounds,
                                                          axis,
                                                          move_positive,
                                                          BlockPos([x, y, z]),
                                                          world
                    );
                    let collide_pos_abs = collide_pos + match axis {
                        0 => x,
                        1 => y,
                        2 => z,
                        _ => unreachable!()
                    } as f64;

                    if (collide_pos_abs < min_collide_pos) ^ !move_positive {
                        min_collide_pos = collide_pos_abs;
                        if (min_collide_pos-self.p[axis]).abs()<1e-3{
                            break 'find_collision;
                        }
                    }
                }
            }
        }
        let collide_time = ((min_collide_pos - move_front) / self.v[axis]).max(0.);
        if collide_time < TICK_TIME {
            self.p[axis] += self.v[axis] * collide_time;
            self.v[axis] = 0.;
            if axis==1 &&!move_positive{
                self.on_ground=true;
            }
        } else {
            self.p[axis] += self.v[axis] * TICK_TIME;
            if axis==1{
                self.on_ground=false;
            }
        }
        if axis==1 && move_positive{
            self.on_ground=false;
        }
    }
}

#[allow(unused_variables)]
fn get_block_collision(block_bounds: [[f64; 2]; 3],
                       move_axis: usize,
                       move_positive: bool,
                       p: BlockPos,
                       world: &WorldReadGuard) -> f64 {
    use block::BlockId;
    use std::f64;
    let empty = world.get_block(&p).map(|b| b == BlockId::empty()).unwrap_or(false);
    if empty {
        if move_positive { f64::INFINITY } else { f64::NEG_INFINITY }
    } else {
        if move_positive { 0. } else { 1. }
    }
}