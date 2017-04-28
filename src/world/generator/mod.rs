use noise::{Perlin, NoiseModule};
use block::BlockId;
use world::random::WorldRngSeeder;
use super::{CHUNK_SIZE, chunk_index, ChunkPos};

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
    noises: [Perlin; 3],
}

const ENV_SCALE: f32 = 1. / 512.;

impl EnvironmentData {
    fn new(seeder: &WorldRngSeeder) -> Self {
        let mut noise_iter = seeder.noises(0);
        let _ = noise_iter.next();//used for random block in main generator
        let s1 = noise_iter.next().unwrap();
        let s2 = noise_iter.next().unwrap();
        let s3 = noise_iter.next().unwrap();
        EnvironmentData {
            noises: [s1, s2, s3],
        }
    }
    pub fn moisture(&self, x: i32, z: i32) -> f32 {
        let temperature = self.temperature(x, z);
        let max_moisture = (temperature * 4.).min(1.);
        max_moisture*(self.noises[2].get([x as f32 * ENV_SCALE, z as f32 * ENV_SCALE]) * 0.5 + 0.5)
    }
    pub fn temperature(&self, x: i32, z: i32) -> f32 {
        let elevation = self.base_elevation(x, z);
        let max_temperature = 1. - (elevation * elevation * 0.5);
        max_temperature * (self.noises[1].get([x as f32 * ENV_SCALE, z as f32 * ENV_SCALE]) * 0.5 + 0.5)
    }
    pub fn base_elevation(&self, x: i32, z: i32) -> f32 {
        self.noises[0].get([x as f32 * ENV_SCALE, z as f32 * ENV_SCALE]) * 0.5 + 0.5
    }
    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        (self.base_elevation(x, z) * 32.) as i32
    }
}

pub struct Generator {
    block_select_noise: Perlin,
    env_data: EnvironmentData,
    blocks: Vec<WorldGenBlock>,
}

impl Generator {
    pub fn new(rand: &WorldRngSeeder, blocks: Vec<WorldGenBlock>) -> Self {
        Generator {
            block_select_noise: rand.noises(0).next().unwrap(),
            env_data: EnvironmentData::new(rand),
            blocks: blocks,
        }
    }

    pub fn env_data(&self)->&EnvironmentData{
        &self.env_data
    }

    pub fn gen_chunk(&self, pos: &ChunkPos) -> [BlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] {
        let base_x = pos[0] * CHUNK_SIZE as i32;
        let base_y = pos[1] * CHUNK_SIZE as i32;
        let base_z = pos[2] * CHUNK_SIZE as i32;
        let mut ret = [BlockId::empty(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
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
                        ret[chunk_index(&[x, y, z])] =
                            self.pick_block(&xz_weights, depth, block_select)
                    }
                }
            }
        }
        ret
    }

    fn pick_block(&self, xz_weights: &[f32], depth: f32, block_select: f32) -> BlockId {
        let mut weight = Vec::with_capacity(self.blocks.len());
        let mut total_weight = 0.;
        assert!(xz_weights.len() == self.blocks.len());
        for i in 0..self.blocks.len() {
            let depth_weight = self.blocks[i].depth.weight(depth);
            weight.push(xz_weights[i] * depth_weight);
            total_weight += xz_weights[i] * depth_weight;
        }
        let mut block_select = block_select * total_weight;
        for i in 0..weight.len() {
            block_select -= weight[i];
            if block_select <= 0. {
                return self.blocks[i].id;
            }
        }
        self.blocks.first().unwrap().id
    }
}
