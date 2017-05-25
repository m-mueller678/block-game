use world::structure::*;
use world::Generator as GeneratorTrait;
use world::*;
use block::AtomicBlockId;
use world::biome::BiomeId;

struct Generator {
    structures: CombinedStructureGenerator,
}

impl GeneratorTrait for Generator {
    fn biome_map(&self, chunk_x: i32, chunk_z: i32) -> [[BiomeId; CHUNK_SIZE]; CHUNK_SIZE] {
        unimplemented!()
    }

    fn surface_y(&self, x: i32, z: i32) -> i32 {
        unimplemented!()
    }

    fn gen_chunk(&self, pos: &ChunkPos) -> ChunkArray<AtomicBlockId> {
        unimplemented!()
    }

    fn reseed(&mut self, _: &WorldRngSeeder) {
        unimplemented!()
    }
}