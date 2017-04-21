use std;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BiomeId(u32);

pub const BIOME_ID_INIT: BiomeId = BiomeId(std::u32::MAX);

pub struct BiomeRegistry {
    biomes: Vec<Biome>,
}

impl BiomeRegistry {
    pub fn new() -> Self {
        BiomeRegistry {
            biomes: Vec::new(),
        }
    }
    pub fn push(&mut self, b: Biome) -> BiomeId {
        self.biomes.push(b);
        BiomeId(self.biomes.len() as u32 - 1)
    }
    pub fn choose_biome(&self, env: &EnvironmentData) -> BiomeId {
        BiomeId(
            self.biomes.iter().map(|biome| {
                biome.environment_data_sq_dist(env)
            }).enumerate().min_by(|&(_, sqd1), &(_, sqd2)| {
                sqd1.partial_cmp(&sqd2).unwrap()
            }).expect("no biomes registered").0 as u32
        )
    }
}

impl std::ops::Index<BiomeId> for BiomeRegistry {
    type Output = Biome;
    fn index(&self, i: BiomeId) -> &Biome {
        &self.biomes[i.0 as usize]
    }
}

pub struct Biome {
    name: String,
    environment: (EnvironmentData, EnvironmentDataWeight),
}

impl Biome {
    pub fn new(name: String,
               ed: EnvironmentData,
               ew: EnvironmentDataWeight
    ) -> Self {
        Biome {
            name: name,
            environment: (ed, ew),
        }
    }
    pub fn environment_data_sq_dist(&self, ed: &EnvironmentData) -> f32 {
        self.environment.1.sq_dist(&self.environment.0, ed)
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct EnvironmentData {
    pub moisture: f32,
    pub temperature: f32,
    pub elevation: f32,
    pub magic: f32,
}

pub struct EnvironmentDataWeight {
    pub moisture: f32,
    pub temperature: f32,
    pub elevation: f32,
    pub magic: f32
}

impl EnvironmentDataWeight {
    pub fn sq_dist(&self, d1: &EnvironmentData, d2: &EnvironmentData) -> f32 {
        (d1.moisture - d2.moisture).powi(2) * self.moisture
            + (d1.temperature - d2.temperature).powi(2) * self.temperature
            + (d1.elevation - d2.elevation).powi(2) * self.elevation
            + (d1.magic - d2.magic).powi(2) * self.magic
    }
}