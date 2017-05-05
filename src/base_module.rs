use std;
use rand::{IsaacRng, Rng};
use num::Integer;
use world::structure::*;
use world::{ChunkPos, EnvironmentData, CHUNK_SIZE, BlockPos, WorldGenBlock,ParameterWeight};
use block::{BlockId, BlockRegistry, Block, LightType};
use graphics::DrawType;
use module::Module;
use block_texture_loader::TextureLoader;

struct BaseModule {}

impl Module for BaseModule {
    fn init(&mut self,
            textures: &mut TextureLoader,
            block_registry: &mut BlockRegistry,
            world_gen_block: &mut FnMut(WorldGenBlock),
            register_structure: &mut FnMut(Box<StructureFinder>)
    ) {
        //blocks
        let block_sand = block_registry.add(Block::new(
            DrawType::FullOpaqueBlock([textures.get("sand"); 6]),
            LightType::Opaque,
            "sand".into()
        ));
        let block_stone = block_registry.add(Block::new(
            DrawType::FullOpaqueBlock([textures.get("stone"); 6]),
            LightType::Opaque,
            "stone".into()
        ));
        let block_dirt = block_registry.add(Block::new(
            DrawType::FullOpaqueBlock([textures.get("dirt"); 6]),
            LightType::Opaque,
            "dirt".into()
        ));
        let block_light = block_registry.add(Block::new(
            DrawType::FullOpaqueBlock([textures.get("debug"); 6]),
            LightType::Source(15),
            "debug_light".into()
        ));

        //world gen blocks
        world_gen_block(WorldGenBlock::new(
            block_dirt,
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(0.5, 1., 0.3, 1.),
            ParameterWeight::new(0., 3., 3., 1.),
        ));
        world_gen_block(WorldGenBlock::new(
            block_sand,
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(0., 0.2, 0.2, 1.),
            ParameterWeight::new(0., 3., 3., 1.),
        ));
        world_gen_block(WorldGenBlock::new(
            block_stone,
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(5., std::f32::INFINITY, 2., 1.),
        ));

        //structures
        register_structure(Box::new(CrossFinder { block: block_light }));
    }
}

pub fn module() -> Box<Module> {
    Box::new(BaseModule {})
}

struct CrossFinder {
    block: BlockId,
}

impl StructureFinder for CrossFinder {
    fn push<'a, 'b, 'c, 'd>(&'a self, chunk: ChunkPos, rand: &'b mut IsaacRng, env_dat: &'c EnvironmentData, out: &'d mut StructureList) {
        let cs = CHUNK_SIZE as i32;
        if rand.gen_weighted_bool(10) {
            let x = chunk[0] * cs + rand.gen_range(0, cs);
            let z = chunk[2] * cs + rand.gen_range(0, cs);
            let surface = env_dat.surface_y(x, z);
            if surface.div_floor(&cs) == chunk[1] {
                out.push(Box::new(CrossGenerator { block: self.block }), BlockPos([x, surface, z]), self.max_bounds());
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
    fn generate<'a>(&self, chunk: &'a mut GeneratingChunk<'a>, _: &mut IsaacRng, _: &EnvironmentData) {
        for i in -10..11 {
            chunk.set_block([i, 0, i], self.block);
            chunk.set_block([i, 0, -i], self.block);
        }
    }
}
