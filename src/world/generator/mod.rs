mod environment_data;
mod surface_map;

use std::collections::HashMap;
use noise::{Perlin, Seedable};
use block::BlockId;
use biome::*;
use world::random::WorldRngSeeder;
use super::{CHUNK_SIZE, chunk_index, ChunkPos, chunk_xz_index};
use self::surface_map::SurfaceMap;

pub use self::surface_map::{SurfaceMapBuilder, BiomeColumnGenerator, BiomeHeightFunction};

pub struct EnvironmentDataWeight {
    pub moisture: f32,
    pub temperature: f32,
    pub elevation: f32,
    pub magic: f32
}

impl EnvironmentDataWeight {
    fn sq_dist(&self, d1: &EnvironmentData, d2: &EnvironmentData) -> f32 {
        (d1.moisture - d2.moisture).powi(2) * self.moisture
            + (d1.temperature - d2.temperature).powi(2) * self.temperature
            + (d1.elevation - d2.elevation).powi(2) * self.elevation
            + (d1.magic - d2.magic).powi(2) * self.magic
    }
}

pub struct Generator {
    seeder: WorldRngSeeder,
    surface_map: SurfaceMap,
    col_gen: HashMap<BiomeId, BiomeColumnGenerator>
}

impl Generator {
    pub fn gen_biome_map(&self, x: i32, z: i32) -> [BiomeId; CHUNK_SIZE * CHUNK_SIZE] {
        let mut ret = [BIOME_ID_INIT; CHUNK_SIZE * CHUNK_SIZE];
        for rx in 0..CHUNK_SIZE {
            for rz in 0..CHUNK_SIZE {
                ret[chunk_xz_index(rx, rz)] = self.surface_map.biome(
                    x * CHUNK_SIZE as i32 + rx as i32,
                    z * CHUNK_SIZE as i32 + rz as i32);
            }
        }
        ret
    }

    pub fn new(rand: WorldRngSeeder, s_map: SurfaceMapBuilder) -> Self {
        let perlin = Perlin::new();
        perlin.set_seed(rand.seed_32() as usize);
        let (s_map, col_gen) = s_map.build();
        Generator {
            seeder: rand,
            surface_map: s_map,
            col_gen: col_gen,
        }
    }

    pub fn gen_chunk(&self, pos: &ChunkPos) -> [BlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] {
        let mut ret = [BlockId::empty(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        let base_x = pos[0] * CHUNK_SIZE as i32;
        let base_z = pos[2] * CHUNK_SIZE as i32;
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let abs_x = x as i32 + base_x;
                let abs_z = z as i32 + base_z;
                let biome_id = self.surface_map.biome(abs_x as i32, abs_z as i32);
                let col_gen = &self.col_gen[&biome_id];
                let relative_y = pos[1] * CHUNK_SIZE as i32 - self.surface_map.height(abs_x, abs_z);
                let col = col_gen(abs_x, abs_z, relative_y, &self.seeder);
                for y in 0..CHUNK_SIZE {
                    ret[chunk_index(&[x, y, z])] = col[y];
                }
            }
        }
        ret
    }
}
