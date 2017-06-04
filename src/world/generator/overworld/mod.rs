use std::sync::{Arc, RwLock};
use noise::Perlin;
use chashmap::*;
use rand::Rng;
use num::Integer;
use world::generator::structure::*;
use world::generator::Generator;
use world::*;
use block::{AtomicBlockId, BlockId};
use world::biome::*;
use world::generator::noise::NoiseParameters;

pub use self::ground_layer_gen::GroundGen;

mod ground_layer_gen;

const BIOME_GEN_CHUNKS: usize = 8;
const BIOME_GEN_SIZE: usize = BIOME_GEN_CHUNKS * CHUNK_SIZE;

type BiomeMap = [[usize; BIOME_GEN_SIZE + 1]; BIOME_GEN_SIZE + 1];
type HeightMap = [[i32; CHUNK_SIZE]; CHUNK_SIZE];

pub struct OverworldGenerator {
    structures: CombinedStructureGenerator,
    biomes: Vec<BiomeId>,
    terrain_parameters: Vec<NoiseParameters>,
    terrain_bases: Vec<i32>,
    ground_layers: Vec<GroundGen>,
    biome_maps: CHashMap<[i32; 2], Box<BiomeMap>>,
    rand: WorldRngSeeder,
    noise_gen: Vec<Perlin>,
    ground: BlockId,
}

impl OverworldGenerator {
    pub fn new(structures: Vec<Box<StructureFinder>>, rand: WorldRngSeeder, ground: BlockId) -> Self {
        OverworldGenerator {
            structures: CombinedStructureGenerator::new(structures, rand),
            biomes: vec![],
            terrain_parameters: vec![],
            terrain_bases: vec![],
            ground_layers: vec![],
            biome_maps: CHashMap::new(),
            rand: rand,
            noise_gen: rand.noises().take(16).collect(),
            ground: ground,
        }
    }

    pub fn add_biome(&mut self, b: BiomeId, terrain: NoiseParameters, terrain_base: i32, mut layers: GroundGen) {
        self.biomes.push(b);
        self.terrain_parameters.push(terrain);
        self.terrain_bases.push(terrain_base);
        layers.reseed(&self.rand);
        self.ground_layers.push(layers);
    }

    fn extract_chunk_biomes(&self, map: &BiomeMap, rel_x: usize, rel_z: usize) -> [[BiomeId; CHUNK_SIZE]; CHUNK_SIZE] {
        let mut ret = [[BiomeId::init(); CHUNK_SIZE]; CHUNK_SIZE];
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                ret[x][z] = self.biomes[map[rel_x + x][rel_z + z]];
            }
        }
        ret
    }

    fn gen_biome_map(&self, x: i32, z: i32) -> Box<BiomeMap> {
        let mut map = Box::new([[0; BIOME_GEN_SIZE + 1]; BIOME_GEN_SIZE + 1]);
        map[0][0] = self.corner_biome(x, z);
        map[0][BIOME_GEN_SIZE] = self.corner_biome(x, z + 1);
        map[BIOME_GEN_SIZE][0] = self.corner_biome(x + 1, z);
        map[BIOME_GEN_SIZE][BIOME_GEN_SIZE] = self.corner_biome(x + 1, z + 1);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[1, x, z]).rng(), &mut |map, i| &mut map[0][i]);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[2, x, z]).rng(), &mut |map, i| &mut map[i][0]);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[3, x + 1, z]).rng(), &mut |map, i| &mut map[BIOME_GEN_SIZE][i]);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[4, x, z + 1]).rng(), &mut |map, i| &mut map[i][BIOME_GEN_SIZE]);
        spread_biomes_from_borders(&mut map, x, z, &self.rand);
        map
    }

    fn read_biome_map<'a>(&'a self, x: i32, z: i32) -> ReadGuard<'a, [i32; 2], Box<BiomeMap>> {
        let bgc = BIOME_GEN_CHUNKS as i32;
        let gen_x = x.div_floor(&bgc);
        let gen_z = z.div_floor(&bgc);
        loop {
            if let Some(read) = self.biome_maps.get(&[gen_x, gen_z]) {
                return read
            }
            self.biome_maps.alter([gen_x, gen_z], |map_opt| {
                if let Some(m) = map_opt {
                    Some(m)
                } else {
                    Some(self.gen_biome_map(gen_x, gen_z))
                }
            });
        }
    }

    fn corner_biome(&self, x: i32, z: i32) -> usize {
        let mut gen = self.rand.push_num(5).pushi(&[x, z]).rng();
        gen.gen_range(0, self.biomes.len())
    }

    fn gen_base_height_map(&self, cx: i32, cz: i32) -> HeightMap {
        let cs = CHUNK_SIZE as i32;
        let mut ret: HeightMap = Default::default();
        let mut reader = BiomeReader::new(&self, cx * cs, cz * cs);
        for x in 0..cs {
            for z in 0..cs {
                let mut y = 0;
                for dx in -10..11 {
                    y += self.terrain_bases[reader.get(cx * cs + x + dx, cz * cs + z)]
                }
                for dz in -10..11 {
                    y += self.terrain_bases[reader.get(cx * cs + x, cz * cs + z + dz)]
                }
                ret[x as usize][z as usize] = y / 42;
            }
        }
        ret
    }

    fn gen_height_map(&self, cx: i32, cz: i32) -> HeightMap {
        let cs = CHUNK_SIZE as i32;
        let mut hm = self.gen_base_height_map(cx, cz);
        let mut reader = BiomeReader::new(&self, cx * cs, cz * cs);
        for x in 0..cs {
            for z in 0..cs {
                let abs_x = cx * cs + x;
                let abs_z = cz * cs + z;
                let biome = reader.get(abs_x, abs_z);
                let biome_nx = reader.get(abs_x + 1, abs_z);
                let biome_nz = reader.get(abs_x, abs_z + 1);
                let height = if biome == biome_nx && biome == biome_nz {
                    self.terrain_parameters[biome]
                        .generate(abs_x as f32, abs_z as f32, self.noise_gen.iter())
                } else {
                    [biome, biome_nx, biome_nz].iter()
                        .map(|b| {
                            self.terrain_parameters[*b]
                                .generate(abs_x as f32, abs_z as f32, self.noise_gen.iter())
                        }).sum::<f32>() / 3.
                };
                hm[x as usize][z as usize] += height.round() as i32;
            }
        }
        hm
    }
}

impl Generator for OverworldGenerator {
    fn biome_map(&self, pos: ChunkPos) -> [[BiomeId; CHUNK_SIZE]; CHUNK_SIZE] {
        let bgc = BIOME_GEN_CHUNKS as i32;
        let rel_x = pos[0].mod_floor(&bgc) as usize;
        let rel_z = pos[2].mod_floor(&bgc) as usize;
        self.extract_chunk_biomes(&self.read_biome_map(pos[0], pos[2]), rel_x, rel_z)
    }

    fn gen_chunk(&self, pos: &ChunkPos) -> ChunkArray<AtomicBlockId> {
        let hm = self.gen_height_map(pos[0], pos[2]);
        let biome_map_quad = self.read_biome_map(pos[0] * CHUNK_SIZE as i32, pos[2] * CHUNK_SIZE as i32);
        let biome_x = pos[0].mod_floor(&(BIOME_GEN_CHUNKS as i32)) as usize * CHUNK_SIZE;
        let biome_z = pos[2].mod_floor(&(BIOME_GEN_CHUNKS as i32)) as usize * CHUNK_SIZE;
        let mut chunk = ChunkArray::<AtomicBlockId>::default();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let biome = biome_map_quad[biome_x + x][biome_z + z];
                let end_index = CHUNK_SIZE - 1;
                let depth = hm[x][z] - (pos[1] * CHUNK_SIZE as i32 + end_index as i32);
                let gen_depth = self.ground_layers[biome].gen_column(depth, &mut |d, block| {
                    chunk[[x, z, end_index - d]] = AtomicBlockId::new(block)
                }, x as i32 + pos[0] * CHUNK_SIZE as i32, z as i32 + pos[2] * CHUNK_SIZE as i32);
                for i in 0..(CHUNK_SIZE - gen_depth) {
                    chunk[[x, z, i]] = AtomicBlockId::new(self.ground);
                }
            }
        }
        chunk
    }

    fn reseed(&mut self, s: &WorldRngSeeder) {
        self.rand = s.clone();
        for l in &mut self.ground_layers {
            l.reseed(s);
        }
        self.noise_gen = s.noises().take(16).collect();
        self.structures.reseed(s);
        self.biome_maps.clear();
    }
}

fn spread_1d<R, I>(min: usize, max: usize, map: &mut BiomeMap, rand: &mut R, index: &mut I)
    where R: Rng,
          I: for<'b> FnMut(&'b mut BiomeMap, usize) -> &'b mut usize, {
    let center_biome = {
        let val = *index(map, if rand.gen() { min } else { max });
        val
    };
    *index(map, (min + max) / 2) = center_biome;
    if max - min > 2 {
        spread_1d(min, (min + max) / 2 + 1, map, rand, index);
        spread_1d((min + max) / 2, max, map, rand, index);
    }
}

fn spread_biomes_from_borders(map: &mut BiomeMap, x: i32, z: i32, seeder: &WorldRngSeeder) {
    assert!(BIOME_GEN_SIZE.count_ones() == 1);//power of two
    let mut size = BIOME_GEN_SIZE;
    let mut rand = seeder.pushi(&[x, z]).rng();
    while size > 2 {
        let mut x = 0;
        while x < BIOME_GEN_SIZE {
            let mut z = 0;
            while z < BIOME_GEN_SIZE {
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
    //don't overwrite borders
    if z1 != 0 {
        map[avg_x][z1] = map[if (rnd & 1) != 0 { x1 } else { x2 }][z1];
    }
    //don't overwrite borders
    if x1 != 0 {
        map[x1][avg_z] = map[x1][if (rnd & 2) != 0 { z1 } else { z2 }];
    }
    map[avg_x][avg_z] = map
        [if (rnd & 4) != 0 { x1 } else { x2 }]
        [if (rnd & 8) != 0 { z1 } else { z2 }];
}

struct BiomeReader<'a> {
    generator: &'a OverworldGenerator,
    guard: ReadGuard<'a, [i32; 2], Box<BiomeMap>>,
    pos: [i32; 2]
}

impl<'a> BiomeReader<'a> {
    fn new(gen: &'a OverworldGenerator, x: i32, z: i32) -> Self {
        let bgs = BIOME_GEN_SIZE as i32;
        let guard = gen.read_biome_map(x, z);
        BiomeReader {
            generator: gen,
            guard: guard,
            pos: [x.div_floor(&bgs), z.div_floor(&bgs)],
        }
    }
    fn get(&mut self, x: i32, z: i32) -> usize {
        let bgs = BIOME_GEN_SIZE as i32;
        let new_pos = [x.div_floor(&bgs), z.div_floor(&bgs)];
        if new_pos != self.pos {
            self.pos = new_pos;
            self.guard = self.generator.read_biome_map(x, z);
        }
        self.guard[x.mod_floor(&bgs) as usize][z.mod_floor(&bgs) as usize]
    }
}