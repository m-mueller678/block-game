use world::{CHUNK_SIZE, chunk_index, ChunkPos};
use block::BlockId;
use num::Integer;
use std::collections::VecDeque;
pub use self::random::WorldRngSeeder;
use rand::Rng;

mod random;

pub struct Generator {
    ground: BlockId,
    height_cache: VecDeque<(i32, i32, Box<HeightMap>)>,
    rand: random::WorldRngSeeder,
}

type HeightMap = [[i32; CHUNK_SIZE]; CHUNK_SIZE];

impl Generator {
    pub fn new(ground: BlockId, rand: WorldRngSeeder) -> Self {
        Generator {
            rand: rand,
            ground: ground,
            height_cache: VecDeque::with_capacity(32),
        }
    }

    pub fn get_height_map(&mut self, x: i32, z: i32) -> usize {
        if let Some(cached) = self.height_cache.iter().position(|&(px, pz, _)| x == px && z == pz) {
            cached
        } else {
            let num_chunks = 8;
            let size = num_chunks * CHUNK_SIZE as i32;
            let mut height_map = Box::new([[0; CHUNK_SIZE]; CHUNK_SIZE]);
            let x_base = x.div_floor(&num_chunks);
            let z_base = z.div_floor(&num_chunks);
            let ch = [
                self.raw_height(x_base, z_base) as i32,
                self.raw_height(x_base + 1, z_base) as i32,
                self.raw_height(x_base + 1, z_base + 1) as i32,
                self.raw_height(x_base, z_base + 1) as i32,
            ];
            let x_mod = x.mod_floor(&num_chunks);
            let z_mod = z.mod_floor(&num_chunks);
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let tx = x as i32 + x_mod * CHUNK_SIZE as i32;
                    let tz = z as i32 + z_mod * CHUNK_SIZE as i32;
                    let avg_neg_z = ch[0] * (size - tx) + ch[1] * tx;
                    let avg_pos_z = ch[3] * (size - tx) + ch[2] * tx;
                    height_map[x][z] = (avg_neg_z * (size - tz) + avg_pos_z * tz) / (4 * size * size)
                }
            }
            if self.height_cache.len() == self.height_cache.capacity() {
                self.height_cache.pop_front();
            }
            self.height_cache.push_back((x, z, height_map));
            self.height_cache.len() - 1
        }
    }

    pub fn gen_chunk(&mut self, pos: &ChunkPos) -> [BlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] {
        let mut ret = [BlockId::empty(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        let height_map_index = self.get_height_map(pos[0], pos[2]);
        let height_map = &self.height_cache[height_map_index];
        let bottom = pos[1] * CHUNK_SIZE as i32;
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let mut y = 0;
                while y < CHUNK_SIZE && y as i32 + bottom < height_map.2[x][z] {
                    ret[chunk_index(&[x, y, z])] = self.ground;
                    y += 1;
                }
            }
        }
        ret
    }

    fn raw_height(&self, x: i32, z: i32) -> u8 {
        self.rand.make_gen(x, z).gen::<u8>()
    }
}