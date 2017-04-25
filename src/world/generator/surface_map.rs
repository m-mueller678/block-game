use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use biome::{BiomeId, EnvironmentData};
use block::BlockId;
use world::{WorldRngSeeder, CHUNK_SIZE};
use world::map_2d::{Map2d, new_map_2d};
use super::EnvironmentDataWeight;
use super::environment_data::Generator as EnvGen;

//x, z, y relative to surface, seeder
pub type BiomeColumnGenerator = Box<Fn(i32, i32, i32, &WorldRngSeeder) -> [BlockId; CHUNK_SIZE] + Send>;
//x, z, elevation environment data, seeder
pub type BiomeHeightFunction = Box<Fn(i32, i32, f32, &WorldRngSeeder) -> i32 + Send>;

pub struct SurfaceMapBuilder {
    biomes: Vec<(BiomeId, EnvironmentData, EnvironmentDataWeight)>,
    // x, z, env data elevation, seeder
    height_functions: HashMap<BiomeId, BiomeHeightFunction>,
    col_gen: HashMap<BiomeId, BiomeColumnGenerator>,
    env_data_noise: EnvGen,
    seeder: WorldRngSeeder,
}

impl SurfaceMapBuilder {
    pub fn new(seeder: WorldRngSeeder) -> Self {
        SurfaceMapBuilder {
            biomes: Vec::new(),
            height_functions: HashMap::new(),
            col_gen: HashMap::new(),
            env_data_noise: EnvGen::new(&seeder),
            seeder: seeder,
        }
    }
    pub fn push_biome(mut self,
                      id: BiomeId,
                      env_data: EnvironmentData,
                      env_weight: EnvironmentDataWeight,
                      height_func: BiomeHeightFunction,
                      col_gen: BiomeColumnGenerator) -> Self {
        let index = self.biomes.binary_search_by_key(&id, |bgen| bgen.0).err().unwrap();
        self.biomes.insert(index, (id, env_data, env_weight));
        self.height_functions.insert(id, height_func).is_none();
        self.col_gen.insert(id, col_gen).is_none();
        self
    }
    pub fn build(self) -> (SurfaceMap, HashMap<BiomeId, BiomeColumnGenerator>) {
        let biome_map = {
            let env_data_noise = self.env_data_noise.clone();
            let biome_vec = self.biomes;
            Arc::new(Mutex::new(new_map_2d(move |x, z| {
                let column_noise_env = env_data_noise.environment_data(x as f32, z as f32);
                (biome_vec.iter().map(|biome_gen| {
                    (biome_gen, biome_gen.2.sq_dist(&biome_gen.1, &column_noise_env))
                }).min_by(|&(_, sq_dist1), &(_, sq_dist2)| {
                    sq_dist1.partial_cmp(&sq_dist2).unwrap()
                }).expect("no biomes registered for generation").0).0
            })))
        };
        let raw_height_map = {
            let height_functions = self.height_functions;
            let biome_map = biome_map.clone();
            let env_data = self.env_data_noise.clone();
            let seeder = self.seeder.clone();
            Arc::new(Mutex::new(new_map_2d(move |x, z| {
                let biome_id = *biome_map.lock().unwrap().get(x, z);
                height_functions[&biome_id](x, z, env_data.elevation(x as f32, z as f32), &seeder)
            })))
        };
        let smooth_height_map = {
            let raw_height = raw_height_map.clone();
            Box::new(Mutex::new(new_map_2d(move |x, z| {
                let mut raw_height = raw_height.lock().unwrap();
                let mut total = 0;
                total += *raw_height.get(x, z);
                total += *raw_height.get(x + 1, z);
                total += *raw_height.get(x, z + 1);
                total += *raw_height.get(x - 1, z);
                total += *raw_height.get(x, z - 1);
                total / 5
            })))
        };
        (SurfaceMap {
            biome: biome_map,
            raw_height: raw_height_map,
            height: smooth_height_map,
        }, self.col_gen)
    }
}


pub struct SurfaceMap {
    biome: Arc<Mutex<Map2d<BiomeId>>>,
    raw_height: Arc<Mutex<Map2d<i32>>>,
    height: Box<Mutex<Map2d<i32>>>,
}

impl SurfaceMap {
    pub fn biome(&self, x: i32, z: i32) -> BiomeId {
        *self.biome.lock().unwrap().get(x, z)
    }
    pub fn raw_height(&self, x: i32, z: i32) -> i32 {
        *self.raw_height.lock().unwrap().get(x, z)
    }
    pub fn height(&self, x: i32, z: i32) -> i32 {
        *self.height.lock().unwrap().get(x, z)
    }
}
