use noise::{Perlin, Seedable};
use rand::{SeedableRng, XorShiftRng, Rng};

#[derive(Copy,Clone)]
pub struct WorldRngSeeder {
    seed: [u32; 4],
    pos: usize,
}

pub type WorldGenRng=XorShiftRng;

impl WorldRngSeeder {
    pub fn new(seed: u64) -> Self {
        WorldRngSeeder {
            seed: [
                seed as u32,
                (seed >> 32) as u32,
                0x12345678,
                0xfedcba98,
            ],
            pos: 0,
        }
    }
    pub fn push_num(&self, n: u32) -> Self {
        let mut ret = self.clone();
        ret.pos %= 4;
        ret.seed[self.pos] ^= n;
        ret.pos += 1;
        ret
    }
    pub fn pushi(&self,source:&[i32])->Self{
        let mut ret=self.clone();
        for s in source{
            ret=ret.push_num(*s as u32);
        }
        ret
    }
    pub fn pushu(&self,source:&[u32])->Self{
        let mut ret=self.clone();
        for s in source{
            ret=ret.push_num(*s);
        }
        ret
    }
    pub fn noises(&self) -> NoiseIterator {
        NoiseIterator { gen: self.rng() }
    }
    pub fn rng(&self) -> WorldGenRng {
        let mut r=XorShiftRng::from_seed(self.seed);
        for _ in 0..4{
            r.next_u32();
        }
        r
    }
}


#[derive(Clone)]
pub struct NoiseIterator {
    gen: WorldGenRng,
}

impl Iterator for NoiseIterator {
    type Item = Perlin;
    fn next(&mut self) -> Option<Perlin> {
        Some(Perlin::new().set_seed(self.gen.gen()))
    }
}
