use world::biome::BiomeId;
use world::*;

pub mod overworld;
pub mod noise;
pub mod structure;

pub trait Generator
where
    Self: Send + Sync,
{
    fn biome_at(&self, x: i32, z: i32) -> BiomeId;
    fn biome_map(&self, pos: ChunkPos) -> [[BiomeId; CHUNK_SIZE]; CHUNK_SIZE];
    fn gen_chunk(&self, pos: ChunkPos) -> Box<ChunkArray<AtomicBlockId>>;
    fn reseed(&mut self, &WorldRngSeeder);
}

pub trait TerrainInformation {
    fn surface_y(&self, x: i32, z: i32) -> i32;
}
