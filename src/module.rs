use std::sync::Arc;
use block_texture_loader::TextureLoader;
use block::{BlockRegistry, BlockId};
use world::generator::noise::NoiseParameters;
use world::generator::overworld::{GroundGen, OverworldGenerator};
use world::generator::structure::StructureFinder;
use world::{WorldRngSeeder, World};
use world::biome::*;

pub struct StartComplete {
    pub biomes: Arc<BiomeRegistry>,
    pub block: Arc<BlockRegistry>,
    pub world: Arc<World>,
    pub textures: TextureLoader,
}

pub fn start<I: Iterator<Item=Box<Init1>>>(init1: I) -> StartComplete {
    let mut block_registry = BlockRegistry::new();
    let mut texture_loader = TextureLoader::new();
    let mut biome_registry = BiomeRegistry::new();
    let i2: Vec<Box<Init2>> = {
        let mut p1 = Phase1 {
            textures: &mut texture_loader,
            blocks: &mut block_registry,
            biomes: &mut biome_registry,
        };
        init1.map(|m: Box<Init1>| {
            m.run(&mut p1)
        }).collect()
    };
    let generator = {
        let mut p2 = Phase2 {
            textures: &texture_loader,
            blocks: &block_registry,
            biomes: &biome_registry,
            gen_biomes: vec![],
            structures: vec![],
        };
        let _:Vec<()>=i2.into_iter().map(|m: Box<Init2>| {
            m.run(&mut p2)
        }).collect();
        p2.build(block_registry.by_name("stone").unwrap(), &WorldRngSeeder::new(42))
    };
    let block_registry = Arc::new(block_registry);
    let biome_registry = Arc::new(biome_registry);
    let world = Arc::new(World::new(block_registry.clone(), Box::new(generator),biome_registry.clone()));
    StartComplete {
        block: block_registry,
        biomes: biome_registry,
        world: world,
        textures: texture_loader,
    }
}

pub struct Phase1<'a> {
    pub textures: &'a mut TextureLoader,
    pub blocks: &'a mut BlockRegistry,
    pub biomes: &'a mut BiomeRegistry,
}

pub struct Phase2<'a> {
    pub textures: &'a TextureLoader,
    pub blocks: &'a BlockRegistry,
    pub biomes: &'a BiomeRegistry,
    gen_biomes: Vec<(BiomeId, NoiseParameters, i32, GroundGen)>,
    structures: Vec<Box<StructureFinder>>,
}

impl<'a> Phase2<'a> {
    pub fn add_overworld_biome(&mut self, b: BiomeId, terrain: NoiseParameters, t_base: i32, layers: GroundGen) {
        self.gen_biomes.push((b, terrain, t_base, layers));
    }
    pub fn add_structure(&mut self, s: Box<StructureFinder>) {
        self.structures.push(s);
    }
    pub fn build(self, ground: BlockId, seeder: &WorldRngSeeder) -> OverworldGenerator {
        let mut gen = OverworldGenerator::new(self.structures, *seeder, ground);
        for (i, t, b, g) in self.gen_biomes {
            gen.add_biome(i, t, b, g);
        }
        gen
    }
}

pub trait Init1 {
    fn run(self: Box<Self>, &mut Phase1) -> Box<Init2>;
}

pub trait Init2 {
    fn run(self: Box<Self>, &mut Phase2);
}

pub trait Module {
    fn init(&self) -> Box<Init1>;
}