use rand::Rng;
use num::Integer;
use block::{BlockId, Block, LightType};
use graphics::DrawType;
use module::*;
use world::*;
use world::generator::structure::*;
use world::generator::*;
use world::generator::noise::NoiseParameters;
use world::generator::overworld::GroundGen;
use world::biome::*;
use geometry::Direction;

struct BaseModule {}

struct InitT1();

struct InitT2();

impl Init1 for InitT1 {
    fn run(self: Box<Self>, p1: &mut Phase1) -> Box<Init2> {
        p1.biomes.register(Biome::new("plain".into()));
        p1.biomes.register(Biome::new("rock".into()));
        p1.blocks.add(Block::new(
            DrawType::FullOpaqueBlock([p1.textures.get("stone"); 6]),
            LightType::Opaque,
            "stone".into()
        ));
        p1.blocks.add(Block::new(
            DrawType::FullOpaqueBlock([p1.textures.get("dirt"); 6]),
            LightType::Opaque,
            "dirt".into()
        ));
        p1.blocks.add(Block::new(
            {
                let mut texture = [p1.textures.get("grass_side"); 6];
                texture[Direction::PosY as usize] = p1.textures.get("grass");
                texture[Direction::NegY as usize] = p1.textures.get("dirt");
                DrawType::FullOpaqueBlock(texture)
            },
            LightType::Opaque,
            "grass".into()
        ));
        p1.blocks.add(Block::new(
            DrawType::FullOpaqueBlock([p1.textures.get("debug"); 6]),
            LightType::Source(15),
            "debug_light".into()
        ));
        Box::new(InitT2())
    }
}

impl Init2 for InitT2 {
    fn run(self: Box<Self>, p2: &mut Phase2) {
        p2.add_overworld_biome(
            p2.biomes.by_name("plain").unwrap(),
            NoiseParameters::new()
                .push(8., 512., None, None)
                .push(4., 128., None, None)
                .push(2., 64., None, None),
            0,
            GroundGen::new()
                .push_layer(p2.blocks.by_name("grass").unwrap(), 1., 1.)
                .push_layer(p2.blocks.by_name("dirt").unwrap(), 1., 4.));
        p2.add_overworld_biome(
            p2.biomes.by_name("rock").unwrap(),
            NoiseParameters::new()
                .push(8., 128., 0., None)
                .push(4., 32., None, None),
            32,
            GroundGen::new()
        );
        p2.add_structure(Box::new(CrossFinder { block: p2.blocks.by_name("debug_light").unwrap() }));
    }
}

impl Module for BaseModule {
    fn init(&self) -> Box<Init1> { Box::new(InitT1 {}) }
}

pub fn module() -> Box<Module> {
    Box::new(BaseModule {})
}

struct CrossFinder {
    block: BlockId,
}

impl StructureFinder for CrossFinder {
    fn push_structures<'a, 'b, 'c>(&'a self, chunk: ChunkPos, rand: &'b WorldRngSeeder, t: &TerrainInformation, out: &'c mut StructureList) {
        let cs = CHUNK_SIZE as i32;
        let mut rand = rand.rng();
        if rand.gen_weighted_bool(10) {
            let x = chunk[0] * cs + rand.gen_range(0, cs);
            let z = chunk[2] * cs + rand.gen_range(0, cs);
            let surface = t.surface_y(x, z);
            if surface.div_floor(&cs) == chunk[1] {
                out.push(
                    Box::new(CrossGenerator { block: self.block }),
                    BlockPos([x, surface, z]),
                    self.max_bounds()
                );
            }
        }
    }
    fn max_bounds(&self) -> [[i32; 2]; 3] {
        [[10, 10], [0, 0], [10, 10]]
    }
}

struct CrossGenerator {
    block: BlockId,
}

impl Structure for CrossGenerator {
    fn generate<'a>(&self, chunk: &'a mut GeneratingChunk<'a>, _: &WorldRngSeeder, _: &TerrainInformation) {
        for i in -10..11 {
            chunk.set_block([i, 0, i], self.block);
            chunk.set_block([i, 0, -i], self.block);
        }
    }
}
