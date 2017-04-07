pub struct Ray {
    start: [f32; 3],
    direction: [f32; 3],
}

impl Ray {
    pub fn new(start: [f32; 3], direction: [f32; 3]) -> Self {
        Ray {
            start: start,
            direction: direction,
        }
    }
    pub fn blocks(&self) -> BlockIntersect {
        let inverse_direction = [
            self.direction[0].recip().abs(),
            self.direction[1].recip().abs(),
            self.direction[2].recip().abs(),
        ];
        let mut fstart = self.start;
        for i in 0..3 {
            if self.direction[i] < 0. {
                fstart[i] = fstart[i].fract();
            } else {
                fstart[i] = 1. - fstart[i].fract();
            }
            fstart[i] *= inverse_direction[i];
        }
        BlockIntersect {
            base: [self.start[0] as i32, self.start[1] as i32, self.start[2] as i32],
            idirection: [
                self.direction[0].signum() as i32,
                self.direction[1].signum() as i32,
                self.direction[2].signum() as i32,
            ],
            fstart: fstart,
            inverse_direction: inverse_direction,
        }
    }
}

pub struct BlockIntersect {
    base: [i32; 3],
    idirection: [i32; 3],
    fstart: [f32; 3],
    inverse_direction: [f32; 3],
}

impl Iterator for BlockIntersect {
    type Item = [i32; 3];
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.base;
        let mut move_axis = 0;
        if self.fstart[1] < self.fstart[move_axis] { move_axis = 1 }
        if self.fstart[2] < self.fstart[move_axis] { move_axis = 2 }
        self.base[move_axis] += self.idirection[move_axis];
        let dist = self.fstart[move_axis];
        for pos in self.fstart.iter_mut() {
            *pos -= dist;
        }
        self.fstart[move_axis] = self.inverse_direction[move_axis];
        Some(ret)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vertical_rays() {
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let ray = Ray::new([x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5], [0., 1., 0.]);
                    let mut blocks = ray.blocks();
                    for i in 0..100 {
                        assert_eq!(blocks.next(), Some([x, y + i, z]));
                    }
                }
            }
        }
    }

    #[test]
    fn diagonal_ray() {
        let ray = Ray::new([0.5, 0., 0.], [1., 1., 0.]);
        let mut blocks = ray.blocks();
        for i in 0..100 {
            assert_eq!(blocks.next(), Some([i, i, 0]));
            assert_eq!(blocks.next(), Some([i, i + 1, 0]));
        }
    }
}