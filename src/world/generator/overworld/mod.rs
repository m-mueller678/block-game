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
use world::generator::TerrainInformation;

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
        let mut map = Box::new([[usize::max_value(); BIOME_GEN_SIZE + 1]; BIOME_GEN_SIZE + 1]);
        map[0][0] = self.corner_biome(x, z);
        map[0][BIOME_GEN_SIZE] = self.corner_biome(x, z + 1);
        map[BIOME_GEN_SIZE][0] = self.corner_biome(x + 1, z);
        map[BIOME_GEN_SIZE][BIOME_GEN_SIZE] = self.corner_biome(x + 1, z + 1);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[1, x, z]).rng(), &mut |map, i| &mut map[0][i]);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[2, x, z]).rng(), &mut |map, i| &mut map[i][0]);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[3, x + 1, z]).rng(), &mut |map, i| &mut map[BIOME_GEN_SIZE][i]);
        spread_1d(0, BIOME_GEN_SIZE, &mut map, &mut self.rand.pushi(&[4, x, z + 1]).rng(), &mut |map, i| &mut map[i][BIOME_GEN_SIZE]);
        for i in 0..BIOME_GEN_SIZE + 1 {
            assert_ne!(map[0][i], usize::max_value());
            assert_ne!(map[i][0], usize::max_value());
            assert_ne!(map[BIOME_GEN_SIZE][i], usize::max_value());
            assert_ne!(map[i][BIOME_GEN_SIZE], usize::max_value());
        }


        spread_biomes(&mut map, &mut self.rand.rng(), 0, 0, BIOME_GEN_SIZE, false, false);
        map
    }

    fn read_biome_map<'a>(&'a self, gen_x: i32, gen_z: i32) -> ReadGuard<'a, [i32; 2], Box<BiomeMap>> {
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

    fn base_height_at(&self, x: i32, z: i32, reader: &mut BiomeReader) -> i32 {
        let mut r=0;
        let mut count=0;
        for dist in &[1,2,3,4,8]{
            for dx in -1..2{
                for dz in -1..2{
                    r+=self.terrain_bases[reader.get(x+dx*dist,z+dz*dist)];
                    count+=1;
                }
            }
        }
        r/count
    }

    fn noise_height_at(&self, x: i32, z: i32, reader: &mut BiomeReader) -> i32 {
        let biome = reader.get(x, z);
        let biome_nx = reader.get(x + 1, z);
        let biome_nz = reader.get(x, z + 1);
        let height = if biome == biome_nx && biome == biome_nz {
            self.terrain_parameters[biome]
                .generate(x as f32, z as f32, self.noise_gen.iter())
        } else {
            [biome, biome_nx, biome_nz].iter()
                .map(|b| {
                    self.terrain_parameters[*b]
                        .generate(x as f32, z as f32, self.noise_gen.iter())
                }).sum::<f32>() / 3.
        };
        height.round() as i32
    }

    fn gen_base_height_map(&self, cx: i32, cz: i32, reader: &mut BiomeReader) -> Box<HeightMap> {
        let cs = CHUNK_SIZE as i32;
        let mut ret: Box<HeightMap> = Default::default();
        for x in 0..cs {
            for z in 0..cs {
                ret[x as usize][z as usize] = self.base_height_at(cx * cs + x, cz * cs + z, reader);
            }
        }
        ret
    }

    fn gen_height_map(&self, cx: i32, cz: i32, reader: &mut BiomeReader) -> Box<HeightMap> {
        let cs = CHUNK_SIZE as i32;
        let mut hm = self.gen_base_height_map(cx, cz, reader);
        for x in 0..cs {
            for z in 0..cs {
                hm[x as usize][z as usize] += self.noise_height_at(cx * cs + x, cz * cs + z, reader);
            }
        }
        hm
    }
}

impl TerrainInformation for OverworldGenerator {
    fn surface_y(&self, x: i32, z: i32) -> i32 {
        let mut reader = BiomeReader::new(&self);
        self.base_height_at(x, z, &mut reader) + self.noise_height_at(x, z, &mut reader)
    }
}

impl Generator for OverworldGenerator {
    fn biome_map(&self, pos: ChunkPos) -> [[BiomeId; CHUNK_SIZE]; CHUNK_SIZE] {
        let bgc = BIOME_GEN_CHUNKS as i32;
        let rel_x = pos[0].mod_floor(&bgc) as usize;
        let rel_z = pos[2].mod_floor(&bgc) as usize;
        self.extract_chunk_biomes(&self.read_biome_map(pos[0].div_floor(&bgc), pos[2].div_floor(&bgc)), rel_x, rel_z)
    }

    fn biome_at(&self, x: i32, z: i32) -> BiomeId {
        let mut reader = BiomeReader::new(self);
        self.biomes[reader.get(x, z)]
    }

    fn gen_chunk(&self, pos: &ChunkPos) -> Box<ChunkArray<AtomicBlockId>> {
        let mut biome_reader = BiomeReader::new(self);
        let hm = self.gen_height_map(pos[0], pos[2], &mut biome_reader);
        let mut chunk = Box::new(ChunkArray::<AtomicBlockId>::default());
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let biome = biome_reader.get(x as i32 + pos[0] * CHUNK_SIZE as i32, z as i32 + pos[2] * CHUNK_SIZE as i32);
                let end_index = CHUNK_SIZE - 1;
                let depth = hm[x][z] - (pos[1] * CHUNK_SIZE as i32 + end_index as i32);
                let gen_depth = self.ground_layers[biome].gen_column(depth, &mut |d, block| {
                    chunk[[x, end_index - d, z, ]] = AtomicBlockId::new(block)
                }, x as i32 + pos[0] * CHUNK_SIZE as i32, z as i32 + pos[2] * CHUNK_SIZE as i32);
                for i in 0..(CHUNK_SIZE - gen_depth) {
                    chunk[[x, i, z]] = AtomicBlockId::new(self.ground);
                }
            }
        }
        self.structures.generate_chunk(*pos, &mut chunk, self);
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
        spread_1d(min, (min + max) / 2, map, rand, index);
        spread_1d((min + max) / 2, max, map, rand, index);
    }
}

fn spread_biomes(map: &mut BiomeMap, rand: &mut WorldGenRng, x: usize, z: usize, s: usize, px: bool, pz: bool) {
    assert_ne!(map[x][z], usize::max_value());
    assert_ne!(map[x + s][z], usize::max_value());
    assert_ne!(map[x][z + s], usize::max_value());
    assert_ne!(map[x + s][z + s], usize::max_value());
    let rnd_val: u8 = rand.gen();
    let hs = s / 2;
    let cx = x + hs;
    let cz = z + hs;
    map[cx][cz] = map[if (rnd_val & 1) != 0 { x } else { x + s }][if (rnd_val & 2) != 0 { z } else { z + s }];
    if px {
        map[x + s][cz] = map[x + s][if (rnd_val & 4) != 0 { z } else { z + s }];
    }
    if pz {
        map[cx][z + s] = map[if (rnd_val & 8) != 0 { x } else { x + s }][z + s];
    }
    if s > 2 {
        spread_biomes(map, rand, x, z, hs, true, true);
        spread_biomes(map, rand, cx, z, hs, px, true);
        spread_biomes(map, rand, x, cz, hs, true, pz);
        spread_biomes(map, rand, cx, cz, hs, px, pz);
    }
}

struct BiomeReader<'a> {
    generator: &'a OverworldGenerator,
    guard: Option<ReadGuard<'a, [i32; 2], Box<BiomeMap>>>,
    pos: [i32; 2]
}

impl<'a> BiomeReader<'a> {
    fn new(gen: &'a OverworldGenerator) -> Self {
        BiomeReader {
            generator: gen,
            guard: None,
            pos: [0, 0],
        }
    }
    fn get(&mut self, x: i32, z: i32) -> usize {
        let bgs = BIOME_GEN_SIZE as i32;
        let new_pos = [x.div_floor(&bgs), z.div_floor(&bgs)];
        if new_pos == self.pos {
            if self.guard.is_none() {
                self.guard = Some(self.generator.read_biome_map(new_pos[0], new_pos[1]));
            }
        } else {
            self.pos = new_pos;
            self.guard = None; //drop old lock
            self.guard = Some(self.generator.read_biome_map(new_pos[0], new_pos[1]));
        }
        self.guard.as_ref().unwrap()[x.mod_floor(&bgs) as usize][z.mod_floor(&bgs) as usize]
    }
}