use rand::{IsaacRng, Rng};
use num::Integer;
use world::structure::*;
use world::{ChunkPos, EnvironmentData, CHUNK_SIZE, BlockPos};
use block::BlockId;

pub fn new_cross_finder(block: BlockId) -> Box<StructureFinder> {
    Box::new(CrossFinder { block: block })
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
