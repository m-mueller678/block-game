use world::generator::noise::NoiseParameters;
use block::BlockId;

pub struct BiomeTerrainGenerator{
    surface:NoiseParameters,
    blocks:Vec<(u32,u32,BlockId)>,
}
