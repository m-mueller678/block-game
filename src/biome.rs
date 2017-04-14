use rand::Rng;
use std;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BiomeId(u32);

pub const BIOME_ID_INIT: BiomeId = BiomeId(std::u32::MAX);

pub struct BiomeRegistry {}

impl BiomeRegistry {
    pub fn choose_rand<R: Rng>(&self, rng: &mut R) -> BiomeId {
        BiomeId(rng.gen_range(1, 20))
    }
}