use noise::{Perlin, Seedable};
use rand::{SeedableRng, IsaacRng, Rng};

#[derive(Clone, Copy)]
pub struct WorldRngSeeder {
    seed: [u32; 2],
}

impl WorldRngSeeder {
    pub fn new(seed: u64) -> Self {
        WorldRngSeeder {
            seed: [
                seed as u32,
                (seed / (u32::max_value() as u64 + 1)) as u32,
            ],
        }
    }
    pub fn make_gen(&self, x: i32, z: i32, y:i32) -> IsaacRng {
        let seed = [
            self.seed[0],
            self.seed[1],
            x as u32,
            z as u32,
            y as u32,
        ];
        let gen = IsaacRng::from_seed(&seed);
        gen
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
        Some(Perlin::new().set_seed(self.gen.gen()))
    }
}
