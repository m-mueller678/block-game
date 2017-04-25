use std;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BiomeId(u32);

impl Default for BiomeId {
    fn default() -> Self {
        BIOME_ID_INIT
    }
}

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
}

impl std::ops::Index<BiomeId> for BiomeRegistry {
    type Output = Biome;
    fn index(&self, i: BiomeId) -> &Biome {
        &self.biomes[i.0 as usize]
    }
}

pub struct Biome {
    name: String,
    environment: EnvironmentData,
}

impl Biome {
    pub fn new(name: String, ed: EnvironmentData) -> Self {
        Biome {
            name: name,
            environment: ed,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn env_data(&self) -> &EnvironmentData {
        &self.environment
    }
}

#[derive(Clone)]
pub struct EnvironmentData {
    pub moisture: f32,
    pub temperature: f32,
    pub elevation: f32,
    pub magic: f32,
}
