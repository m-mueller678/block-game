use world::structure::*;
use world::Generator as GeneratorTrait;
use world::*;
use block::AtomicBlockId;

struct Generator {
    structures: CombinedStructureGenerator,
}

impl GeneratorTrait for Generator {
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