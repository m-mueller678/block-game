use noise::{Perlin, Seedable};
use rand::{SeedableRng, IsaacRng, Rng};

#[derive(Clone, Copy)]
pub struct WorldRngSeeder {
    seed: [u32; 2],
}

impl WorldRngSeeder {
    pub fn new(seed: u64) -> Self {
        use std::mem::transmute;
        let seed = unsafe {
            transmute::<u64, [u32; 2]>(u64::to_le(seed))
        };
        WorldRngSeeder {
            seed: [u32::from_le(seed[0]), u32::from_le(seed[1])],
        }
    }
    pub fn make_gen(&self, x: i32, z: i32) -> impl Rng {
        let seed = [
            self.seed[0],
            self.seed[1],
            x as u32,
            z as u32,
        ];
        let gen = IsaacRng::from_seed(&seed);
        gen
    }
    pub fn seed_32(&self) -> u32 {
        self.seed[0]
    }
    pub fn noises(&self, i: u32) -> NoiseIterator {
        let seed = [self.seed[0], self.seed[1], i];
        NoiseIterator { gen: IsaacRng::from_seed(&seed) }
    }
}

#[derive(Clone)]
pub struct NoiseIterator {
    gen: IsaacRng,
}

impl Iterator for NoiseIterator {
    type Item = Perlin;
    fn next(&mut self) -> Option<Perlin> {
        let p = Perlin::new();
        p.set_seed(self.gen.gen());
        Some(p)
    }
}
