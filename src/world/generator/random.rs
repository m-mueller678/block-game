use rand::{SeedableRng, IsaacRng, Rng};

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
}
