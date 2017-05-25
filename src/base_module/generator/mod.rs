use std::sync::{Arc, RwLock};
use chashmap::CHashMap;
use rand::Rng;
use num::Integer;
use world::structure::*;
use world::Generator as GeneratorTrait;
use world::*;
use block::AtomicBlockId;
use world::biome::*;

const BIOME_GEN_CHUNKS: usize = 8;
const BIOME_GEN_SIZE: usize = BIOME_GEN_CHUNKS * CHUNK_SIZE;

type BiomeMap = [[BiomeId; BIOME_GEN_SIZE + 1]; BIOME_GEN_SIZE + 1];

struct Generator {
    structures: CombinedStructureGenerator,
    biomes: Vec<BiomeId>,
    biome_maps: CHashMap<[i32; 2], BiomeMap>,
    seeder: WorldRngSeeder,
}

impl Generator{
    pub fn new(structures:Vec<Box<StructureFinder>>,biomes:Vec<BiomeId>)->Self{
        Generator{
            structures:CombinedStructureGenerator::new(structures,WorldRngSeeder::new(0)),
            biomes:biomes,
            biome_maps:CHashMap::new(),
            seeder:WorldRngSeeder::new(0),
        }
    }
}

impl GeneratorTrait for Generator {
    fn biome_map(&self, chunk_x: i32, chunk_z: i32) -> [[BiomeId; CHUNK_SIZE]; CHUNK_SIZE] {
        let bgc = BIOME_GEN_CHUNKS as i32;
        let gen_x = chunk_x.div_floor(&bgc);
        let gen_z = chunk_z.div_floor(&bgc);
        let rel_x = chunk_x.mod_floor(&bgc) as usize;
        let rel_z = chunk_z.mod_floor(&bgc) as usize;
        loop {
            if let Some(read) = self.biome_maps.get(&[gen_x, gen_z]) {
                return extract_chunk_biomes(&*read, rel_x, rel_z);
            }

            self.biome_maps.alter([gen_x, gen_z], |map_opt| {
                if let Some(m) = map_opt {
                    Some(m)
                } else {
                    Some(gen_biome_map(gen_x, gen_z, &self.seeder, &self.biomes))
                }
            })
        }
    }

    fn gen_chunk(&self, pos: &ChunkPos) -> ChunkArray<AtomicBlockId> {
        unimplemented!()
    }

    fn reseed(&mut self, s: &WorldRngSeeder) {
        self.seeder=s.clone();
        self.structures.reseed(s);
        self.biome_maps.clear();
    }

    fn surface_y(&self, x: i32, z: i32) -> i32 {
        unimplemented!()
    }
}

fn extract_chunk_biomes(map: &BiomeMap, rel_x: usize, rel_z: usize) -> [[BiomeId; CHUNK_SIZE]; CHUNK_SIZE] {
    let mut ret = [[BiomeId::init(); CHUNK_SIZE]; CHUNK_SIZE];
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            ret[x][z] = map[rel_x + x][rel_z + z];
        }
    }
    ret
}

fn gen_biome_map(x: i32, z: i32, seeder: &WorldRngSeeder, b: &[BiomeId]) -> BiomeMap {
    let mut map = [[BiomeId::init(); BIOME_GEN_SIZE + 1]; BIOME_GEN_SIZE + 1];
    map[0][0] = corner_biome(x, z, seeder, b);
    map[0][BIOME_GEN_SIZE + 1] = corner_biome(x, z + 1, seeder, b);
    map[BIOME_GEN_SIZE + 1][0] = corner_biome(x + 1, z, seeder, b);
    map[BIOME_GEN_SIZE + 1][BIOME_GEN_SIZE + 1] = corner_biome(x + 1, z + 1, seeder, b);
    spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut seeder.make_gen(x, z, 0), &mut |map, i| &mut map[0][i]);
    spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut seeder.make_gen(x, z, 1), &mut |map, i| &mut map[i][0]);
    spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut seeder.make_gen(x + 1, z, 0), &mut |map, i| &mut map[BIOME_GEN_SIZE][i]);
    spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut seeder.make_gen(x, z + 1, 1), &mut |map, i| &mut map[i][BIOME_GEN_SIZE]);
    spread_biomes_from_borders(&mut map, x, z, seeder);
    map
}

fn corner_biome(x: i32, z: i32, seeder: &WorldRngSeeder, b: &[BiomeId]) -> BiomeId {
    let mut gen = seeder.make_gen(x, z, 1);
    *gen.choose(b).unwrap()
}

fn spread_1d<R, I>(min: usize, max: usize, map: &mut BiomeMap, rand: &mut R, index: &mut I)
    where R: Rng,
          I: for<'b> FnMut(&'b mut BiomeMap, usize) -> &'b mut BiomeId, {
    let center_biome = {
        let val = *index(map, if rand.gen() { min } else { max });
        val
    };
    *index(map, min + max) = center_biome;
    if max - min > 2 {
        spread_1d(min, (min + max) / 2 + 1, map, rand, index);
        spread_1d((min + max) / 2, max, map, rand, index);
    }
}

fn spread_biomes_from_borders(map: &mut BiomeMap, x: i32, z: i32, seeder: &WorldRngSeeder) {
    assert!(BIOME_GEN_SIZE.count_ones() == 1);//power of two
    let mut size = BIOME_GEN_SIZE;
    let mut rand = seeder.make_gen(x, z, 0);
    while size > 2 {
        let mut x = 0;
        while x < BIOME_GEN_CHUNKS * CHUNK_SIZE {
            let mut z = 0;
            while x < BIOME_GEN_CHUNKS * CHUNK_SIZE {
                spread_biomes(map, x, x + size, z, z + size, &mut rand);
                z += size
            }
            x += size;
        }
        size /= 2;
    }
}

fn spread_biomes<R: Rng>(map: &mut BiomeMap, x1: usize, x2: usize, z1: usize, z2: usize, rand: &mut R) {
    let rnd: u8 = rand.gen();
    let avg_x = (x1 + x2) / 2;
    let avg_z = (z1 + z2) / 2;
    if z1 != 0 {
        //don't overwrite borders
        map[avg_x][z1] = map[if (rnd & 1) != 0 { x1 } else { x2 }][z1];
    }
    if x1 != 0 {
        //don't overwrite borders
        map[x1][avg_z] = map[x1][if (rnd & 2) != 0 { z1 } else { z2 }];
    }
    map[avg_x][avg_z] = map
        [if (rnd & 4) != 0 { x1 } else { x2 }]
        [if (rnd & 8) != 0 { z1 } else { z2 }];
}
