use std::collections::BTreeMap;
use std::ops::Index;

pub struct Biome {
    name: String,
}

impl Biome {
    pub fn new(name: String) -> Self {
        Biome { name: name }
    }
    pub fn name(&self) -> &str {
        &self.name
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
    by_name: BTreeMap<String, BiomeId>,
}

impl BiomeRegistry {
    pub fn new() -> Self {
        BiomeRegistry {
            biomes: vec![],
            by_name: Default::default(),
        }
    }

    pub fn register(&mut self, b: Biome) -> BiomeId {
        let id = BiomeId(self.biomes.len() as u32);
        let same_name = self.by_name.insert(b.name.clone(), id);
        self.biomes.push(b);
        assert!(same_name.is_none());
        id
    }

    pub fn by_name(&self, name: &str) -> Option<BiomeId> {
        self.by_name.get(name).cloned()
    }
}

impl Index<BiomeId> for BiomeRegistry {
    type Output = Biome;
    fn index(&self, id: BiomeId) -> &Biome {
        &self.biomes[id.0 as usize]
    }
}
