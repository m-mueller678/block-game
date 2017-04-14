use world::{CHUNK_SIZE, WorldRngSeeder};
use biome::{BiomeId, BIOME_ID_INIT, BiomeRegistry};
use rand::Rng;
use super::super::chunk::chunk_xz_index;
use std::sync::Arc;
use super::BiomeMap;
use num::Integer;

pub struct BiomeGenerator {
    zone_size: i32,
    rand: WorldRngSeeder,
    biomes: Arc<BiomeRegistry>,
}

type NodeList = Vec<(i32, i32, BiomeId)>;

impl BiomeGenerator {
    pub fn new(zone_size: i32, seeder: WorldRngSeeder, biomes: Arc<BiomeRegistry>) -> Self {
        assert!(zone_size >= CHUNK_SIZE as i32);
        assert!(zone_size % CHUNK_SIZE as i32 == 0);
        BiomeGenerator {
            zone_size: zone_size,
            rand: seeder,
            biomes: biomes,
        }
    }
    pub fn gen_chunk(&self, chunk_x: i32, chunk_z: i32) -> BiomeMap {
        let zone_x = (chunk_x * CHUNK_SIZE as i32).div_floor(&self.zone_size);
        let zone_z = (chunk_z * CHUNK_SIZE as i32).div_floor(&self.zone_size);
        let mut nodes = NodeList::new();
        let mut range = 1;
        self.push_biome_nodes(zone_x, zone_z, &mut nodes);
        self.push_surrounding_zones(zone_x, zone_z, range, &mut nodes);
        let mut ret = [BIOME_ID_INIT; CHUNK_SIZE * CHUNK_SIZE];
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let abs_x = chunk_x * CHUNK_SIZE as i32 + x as i32;
                let abs_z = chunk_z * CHUNK_SIZE as i32 + z as i32;
                let mut biome = Self::find_closest(abs_x, abs_z, &nodes, (range * self.zone_size).pow(2));
                while biome.is_none() {
                    range += 1;
                    self.push_surrounding_zones(zone_x, zone_z, range, &mut nodes);
                    biome = Self::find_closest(abs_x, abs_z, &nodes, (range * self.zone_size - 1).pow(2));
                }
                ret[chunk_xz_index(x, z)] = biome.unwrap();
            }
        }
        ret
    }
    fn push_surrounding_zones(&self, x: i32, z: i32, r: i32, list: &mut NodeList) {
        for dx in -r..(r + 1) {
            self.push_biome_nodes(x + dx, z - r, list)
        }
        for dx in -r..(r + 1) {
            self.push_biome_nodes(x + dx, z + r, list)
        }
        for dz in (-r + 1)..(r) {
            self.push_biome_nodes(x - r, z + dz, list)
        }
        for dz in (-r + 1)..(r) {
            self.push_biome_nodes(x + r, z + dz, list)
        }
    }
    fn push_biome_nodes(&self, x: i32, z: i32, vec: &mut NodeList) {
        let mut rand = self.rand.make_gen(x, z);
        vec.push((x * self.zone_size, z * self.zone_size, self.biomes.choose_rand(&mut rand)));
        if rand.gen() {
            let base_x = x * self.zone_size;
            let base_z = z * self.zone_size;
            let nx = rand.gen_range(0, self.zone_size);
            let nz = rand.gen_range(0, self.zone_size);
            vec.push((nx + base_x, nz + base_z, self.biomes.choose_rand(&mut rand)));
        }
    }
    fn find_closest(x: i32, z: i32, nodes: &NodeList, max_sq_dist: i32) -> Option<BiomeId> {
        let mut best_sq_dist = max_sq_dist;
        let mut ret = None;
        for node in nodes.iter() {
            let sq_dist = (node.0 - x).pow(2) + (node.1 - z).pow(2);
            if sq_dist < best_sq_dist {
                best_sq_dist = sq_dist;
                ret = Some(node.2);
            }
        }
        ret
    }
}