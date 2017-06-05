use world::biome::BiomeId;
use world::*;
use block::BlockId;

pub mod overworld;
pub mod noise;
pub mod structure;

pub trait Generator where Self:Send+Sync{
    fn biome_map(&self,pos:ChunkPos)->[[BiomeId;CHUNK_SIZE];CHUNK_SIZE];
    fn gen_chunk(&self, pos: &ChunkPos) -> Box<ChunkArray<AtomicBlockId>>;
    fn reseed(&mut self,&WorldRngSeeder);
}

pub struct TerrainInformation{
    surface:[[i32;CHUNK_SIZE];CHUNK_SIZE],
}

impl TerrainInformation{
    pub fn abs_surface_y(&self,x:usize,z:usize)->i32{
        self.surface[x][z]
    }
}