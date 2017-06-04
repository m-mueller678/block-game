use std;
use rand::{IsaacRng, Rng};
use num::Integer;
use block::{BlockId, BlockRegistry, Block, LightType};
use graphics::DrawType;
use module::*;
use block_texture_loader::TextureLoader;
use world::*;
use world::generator::structure::*;
use world::generator::*;
use world::generator::noise::NoiseParameters;
use world::generator::overworld::GroundGen;
use world::biome::*;

struct BaseModule {}

struct InitT1 ();
struct InitT2 (BiomeId,BlockId);

impl Init1 for InitT1 {
    fn run(self: Box<Self>, p1: &mut Phase1) -> Box<Init2> {
        let biome1=p1.biomes.register(Biome::new());
        let block_sand = p1.blocks.add(Block::new(
            DrawType::FullOpaqueBlock([p1.textures.get("sand"); 6]),
            LightType::Opaque,
            "sand".into()
        ));
        let block_stone = p1.blocks.add(Block::new(
            DrawType::FullOpaqueBlock([p1.textures.get("stone"); 6]),
            LightType::Opaque,
            "stone".into()
        ));
        let block_dirt = p1.blocks.add(Block::new(
            DrawType::FullOpaqueBlock([p1.textures.get("dirt"); 6]),
            LightType::Opaque,
            "dirt".into()
        ));
        let block_light = p1.blocks.add(Block::new(
            DrawType::FullOpaqueBlock([p1.textures.get("debug"); 6]),
            LightType::Source(15),
            "debug_light".into()
        ));
        Box::new(InitT2(biome1,block_dirt))
    }
}

impl Init2 for InitT2 {
    fn run(self: Box<Self>, p2: &mut Phase2) {
        p2.add_overworld_biome(self.0,NoiseParameters::new().push(32.,512.,None,None),0,GroundGen::new().push_layer(self.1,1.,3.))
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
    fn push_structures<'a, 'b, 'c, 'd>(&'a self, chunk: ChunkPos, rand: &'b WorldRngSeeder, t: &TerrainInformation, out: &'d mut StructureList) {
        let cs = CHUNK_SIZE as i32;
        let mut rand = rand.rng();
        if rand.gen_weighted_bool(10) {
            let x = rand.gen_range(0, CHUNK_SIZE);
            let z = rand.gen_range(0, CHUNK_SIZE);
            let surface = t.abs_surface_y(x, z);
            if surface.div_floor(&cs) == chunk[1] {
                out.push(
                    Box::new(CrossGenerator { block: self.block }),
                    BlockPos([
                        chunk[0] * cs + x as i32,
                        surface,
                        chunk[2] * cs + z as i32
                    ]),
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
