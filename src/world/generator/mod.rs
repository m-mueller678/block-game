use std::sync::Arc;
use noise::{Perlin, Seedable, NoiseModule};
use super::{CHUNK_SIZE, chunk_index, ChunkPos, chunk_xz_index};
use block::BlockId;
use biome::*;
pub use self::random::WorldRngSeeder;
use self::environment_data::Generator as EnvGen;

mod random;
mod environment_data;

pub type BiomeMap = [BiomeId; CHUNK_SIZE * CHUNK_SIZE];

pub struct Generator {
    ground: BlockId,
    perlin: Perlin,
    env_data: EnvGen,
    biomes: Arc<BiomeRegistry>,
}


impl Generator {
    pub fn new(ground: BlockId, rand: WorldRngSeeder, biomes: Arc<BiomeRegistry>) -> Self {
        let perlin = Perlin::new();
        perlin.set_seed(rand.seed_32() as usize);
        Generator {
            ground: ground,
            perlin: perlin,
            env_data: EnvGen::new(&rand),
            biomes: biomes,
        }
    }

    pub fn gen_biome_map(&self, x: i32, z: i32) -> BiomeMap {
        let mut ret = [BIOME_ID_INIT; CHUNK_SIZE * CHUNK_SIZE];
        let x_base = x as f32 * CHUNK_SIZE as f32;
        let z_base = z as f32 * CHUNK_SIZE as f32;
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let abs_noise_x = x as f32 + x_base;
                let abs_noise_z = z as f32 + z_base;
                ret[chunk_xz_index(x, z)] =
                    self.biomes.choose_biome(&self.env_data.environment_data(abs_noise_x, abs_noise_z));
            }
        }
        ret
    }

    pub fn gen_chunk(&mut self, pos: &ChunkPos) -> [BlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] {
        let base_x = pos[0] as f64 * CHUNK_SIZE as f64;
        let base_z = pos[2] as f64 * CHUNK_SIZE as f64;
        if pos[1] < 0 {
            [self.ground; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE]
        } else if pos[1] > 0 {
            [BlockId::empty(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE]
        } else {
            let mut ret = [BlockId::empty(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let perlin = self.perlin.get([(base_x + x as f64) / 128., (base_z + z as f64) / 128.]);
                    let perlin = (perlin + 1.) * 0.5;
                    let perlin = (perlin * perlin) * (perlin * -2. + 3.);
                    let height = (perlin * CHUNK_SIZE as f64) as usize;
                    for y in 0..height {
                        ret[chunk_index(&[x, y, z])] = self.ground;
                    }
                }
            }
            ret
        }
    }
}