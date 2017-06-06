pub struct Biome {}

impl Biome{
    pub fn new()->Self{
        Biome{}
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BiomeId(u32);

impl BiomeId {
    pub fn init() -> Self {
        BiomeId(u32::max_value())
    }
}

pub struct BiomeRegistry {
    biomes: Vec<Biome>,
    ids:Vec<BiomeId>,
}

impl BiomeRegistry {
    pub fn new() -> Self {
        BiomeRegistry {
            biomes: vec![],
            ids:vec![],
        }
    }

    pub fn register(&mut self, b: Biome) -> BiomeId {
        let id=BiomeId(self.biomes.len() as u32);
        self.ids.push(id);
        self.biomes.push(b);
        BiomeId(self.biomes.len() as u32)
    }
}

