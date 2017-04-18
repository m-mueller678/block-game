use noise::{Perlin, Seedable, NoiseModule};
use world::{CHUNK_SIZE, chunk_index, ChunkPos};
use block::BlockId;
use biome::*;
pub use self::random::WorldRngSeeder;
use std::sync::Arc;

mod random;
mod biome;

pub type BiomeMap = [BiomeId; CHUNK_SIZE * CHUNK_SIZE];

pub struct Generator {
    ground: BlockId,
    biome_generator: biome::BiomeGenerator,
    perlin: Perlin
}


impl Generator {
    pub fn new(ground: BlockId, rand: WorldRngSeeder, biomes: Arc<BiomeRegistry>) -> Self {
        let perlin = Perlin::new();
        perlin.set_seed(rand.seed_32() as usize);
        Generator {
            ground: ground,
            biome_generator: biome::BiomeGenerator::new(256, rand, biomes),
            perlin: perlin
        }
    }

    pub fn gen_biome_map(&self, x: i32, z: i32) -> BiomeMap {
        self.biome_generator.gen_chunk(x, z)
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