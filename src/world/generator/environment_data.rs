use noise::{Perlin, NoiseModule, Seedable};
use super::WorldRngSeeder;
use rand::Rng;
use biome::EnvironmentData;

#[derive(Clone)]
pub struct Generator {
    noise: [Perlin; 4],
}

const MOISTURE_SCALE: f32 = 1. / 128.;
const TEMPERATURE_SCALE: f32 = 1. / 128.;
const ELEVATION_SCALE: f32 = 1. / 128.;
const MAGIC_SCALE: f32 = 1. / 128.;

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
        self.noise[0].get([x * MOISTURE_SCALE, z * MOISTURE_SCALE])
    }
    pub fn temperature(&self, x: f32, z: f32) -> f32 {
        self.noise[1].get([x * TEMPERATURE_SCALE, z * TEMPERATURE_SCALE])
    }
    pub fn elevation(&self, x: f32, z: f32) -> f32 {
        self.noise[2].get([x * ELEVATION_SCALE, z * ELEVATION_SCALE])
    }
    pub fn magic(&self, x: f32, z: f32) -> f32 {
        self.noise[3].get([x * MAGIC_SCALE, z * MAGIC_SCALE])
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