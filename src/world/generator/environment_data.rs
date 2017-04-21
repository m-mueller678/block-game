use noise::{Perlin, NoiseModule, Seedable};
use super::WorldRngSeeder;
use rand::Rng;
use biome::EnvironmentData;

pub struct Generator {
    noise: [Perlin; 4],
}

impl Generator {
    pub fn new(s: &WorldRngSeeder) -> Self {
        let mut rand = s.make_gen(0, 0);
        let mut ret = Generator { noise: [Perlin::new(); 4] };
        for noise in ret.noise.iter_mut() {
            noise.set_seed(rand.gen());
        }
        ret
    }
    pub fn moisture(&self, x: f32, z: f32) -> f32 {
        self.noise[0].get([x, z])
    }
    pub fn temperature(&self, x: f32, z: f32) -> f32 {
        self.noise[1].get([x, z])
    }
    pub fn elevation(&self, x: f32, z: f32) -> f32 {
        self.noise[2].get([x, z])
    }
    pub fn magic(&self, x: f32, z: f32) -> f32 {
        self.noise[3].get([x, z])
    }
    pub fn environment_data(&self, x: f32, z: f32) -> EnvironmentData {
        EnvironmentData {
            moisture: self.moisture(x, z),
            temperature: self.temperature(x, z),
            elevation: self.elevation(x, z),
            magic: self.magic(x, z),
        }
    }
}