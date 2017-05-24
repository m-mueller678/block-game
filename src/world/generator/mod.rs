use noise::{Perlin, NoiseModule};
use block::{AtomicBlockId, BlockId};
use world::random::WorldRngSeeder;
use super::{CHUNK_SIZE, ChunkPos, ChunkArray};

pub mod structure;

use self::structure::{CombinedStructureGenerator,StructureFinder};

#[derive(Clone)]
pub struct ParameterWeight {
    min: f32,
    max: f32,
    weight: f32,
    width: f32,
}

impl ParameterWeight {
    pub fn new(min: f32, max: f32, width: f32, weight: f32) -> Self {
        ParameterWeight { min: min, max: max, width: width, weight: weight }
    }
    fn weight(&self, val: f32) -> f32 {
        if val < self.min {
            self.weight_at_dist(self.min - val)
        } else if val > self.max {
            self.weight_at_dist(val - self.max)
        } else {
            self.weight
        }
    }
    fn weight_at_dist(&self, d: f32) -> f32 {
        (self.width - d).max(0.) / self.width * self.weight
    }
}

#[derive(Clone)]
pub struct WorldGenBlock {
    id: BlockId,
    temperature: ParameterWeight,
    moisture: ParameterWeight,
    depth: ParameterWeight,
}

impl WorldGenBlock {
    pub fn new(id: BlockId,
               temperature: ParameterWeight,
               moisture: ParameterWeight,
               depth: ParameterWeight)
               -> Self {
        WorldGenBlock { id: id, temperature: temperature, moisture: moisture, depth: depth }
    }
}

#[derive(Clone)]
pub struct EnvironmentData {
    elevation_base: Vec<Perlin>,
    moisture: Vec<Perlin>,
    temperature: Vec<Perlin>,
}

const ENV_SCALE: f32 = 1. / 512.;
const ELEVATION_BASE_LAYERS: usize = 6;
const MOISTURE_LAYERS: usize = 3;
const TEMPERATURE_LAYERS: usize = 3;

impl EnvironmentData {
    fn new(seeder: &WorldRngSeeder) -> Self {
        let mut noises = seeder.noises(0);
        let _ = noises.next();//used for random block in main generator
        let mut ed = EnvironmentData {
            elevation_base: Vec::with_capacity(ELEVATION_BASE_LAYERS),
            moisture: Vec::with_capacity(MOISTURE_LAYERS),
            temperature: Vec::with_capacity(TEMPERATURE_LAYERS),
        };
        for _ in 0..ELEVATION_BASE_LAYERS { ed.elevation_base.push(noises.next().unwrap()) }
        for _ in 0..MOISTURE_LAYERS { ed.moisture.push(noises.next().unwrap()) }
        for _ in 0..TEMPERATURE_LAYERS { ed.temperature.push(noises.next().unwrap()) }
        ed
    }
    pub fn moisture(&self, x: i32, z: i32) -> f32 {
        let temperature = self.temperature(x, z);
        let max_moisture = (temperature * 4.).min(1.);
        max_moisture * Self::make_noise(&self.moisture, ENV_SCALE, x as f32, z as f32)
    }
    pub fn temperature(&self, x: i32, z: i32) -> f32 {
        let elevation = self.base_elevation(x, z);
        let max_temperature = 1. - (elevation * elevation * 0.5);
        max_temperature * Self::make_noise(&self.temperature, ENV_SCALE, x as f32, z as f32)
    }
    pub fn base_elevation(&self, x: i32, z: i32) -> f32 {
        Self::make_noise(&self.elevation_base, ENV_SCALE, x as f32, z as f32)
    }
    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        (self.base_elevation(x, z) * 64.) as i32
    }
    fn make_noise(noises: &[Perlin], base_scale: f32, x: f32, z: f32) -> f32 {
        let mut val_scale = 1.;
        let mut pos_scale = base_scale;
        let mut max_abs = 0.;
        let mut total = 0.;
        for n in noises {
            total += n.get([x * pos_scale, z * pos_scale]) * val_scale;
            max_abs += val_scale;
            val_scale *= 0.7;
            pos_scale *= 2.;
        }
        (total / max_abs * 0.5 + 0.5)
    }
}

pub struct Generator {
    block_select_noise: Perlin,
    env_data: EnvironmentData,
    blocks: Vec<WorldGenBlock>,
    structures:CombinedStructureGenerator,
}

impl Generator {
    pub fn new(rand: &WorldRngSeeder, blocks: Vec<WorldGenBlock>,structures:Vec<Box<StructureFinder>>) -> Self {
        let env_dat=EnvironmentData::new(rand);
        Generator {
            block_select_noise: rand.noises(0).next().unwrap(),
            env_data: env_dat.clone(),
            blocks: blocks,
            structures:CombinedStructureGenerator::new(structures,rand.clone(),env_dat),
        }
    }

    pub fn env_data(&self) -> &EnvironmentData {
        &self.env_data
    }

    pub fn gen_chunk(&self, pos: &ChunkPos) -> ChunkArray<AtomicBlockId> {
        let base_x = pos[0] * CHUNK_SIZE as i32;
        let base_y = pos[1] * CHUNK_SIZE as i32;
        let base_z = pos[2] * CHUNK_SIZE as i32;
        let mut ret: ChunkArray<AtomicBlockId> = Default::default();
        let mut weight_buffer = vec![0.; self.blocks.len()];
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let abs_x = x as i32 + base_x;
                let abs_z = z as i32 + base_z;
                let surface_y = self.env_data.surface_y(abs_x, abs_z);
                if surface_y > base_y {
                    let moisture = self.env_data.moisture(abs_x, abs_z);
                    let temperature = self.env_data.temperature(abs_x, abs_z);
                    let block_select = self.block_select_noise.get([abs_x as f32 * ENV_SCALE / 16., abs_z as f32 * ENV_SCALE / 16.]) * 0.5 + 0.5;
                    let xz_weights: Vec<f32> = self.blocks.iter()
                        .map(|gen_block| {
                            gen_block.moisture.weight(moisture)
                                * gen_block.temperature.weight(temperature)
                        }).collect();
                    for y in 0..CHUNK_SIZE {
                        let abs_y = y as i32 + base_y;
                        if abs_y >= surface_y {
                            break;
                        }
                        let depth = (surface_y - abs_y) as f32;
                        ret[[x, y, z]]=AtomicBlockId::new(
                            (self.pick_block(&xz_weights, depth, block_select, &mut weight_buffer))
                        );
                    }
                }
            }
        }
        self.structures.generate_chunk(*pos,&mut ret);
        ret
    }

    fn pick_block(&self, xz_weights: &[f32], depth: f32, block_select: f32, weight_buffer: &mut [f32]) -> BlockId {
        let mut total_weight = 0.;
        for i in 0..self.blocks.len() {
            let depth_weight = self.blocks[i].depth.weight(depth);
            weight_buffer[i] = xz_weights[i] * depth_weight;
            total_weight += xz_weights[i] * depth_weight;
        }
        let mut block_select = block_select * total_weight;
        for i in 0..self.blocks.len() {
            block_select -= weight_buffer[i];
            if block_select <= 0. {
                return self.blocks[i].id;
            }
        }
        self.blocks.first().unwrap().id
    }
}
