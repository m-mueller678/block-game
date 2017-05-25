use std::ops::Range;
use std::cmp;
use vecmath::*;
use chashmap::*;
use num::Integer;
use world::{CHUNK_SIZE, ChunkArray, ChunkPos, BlockPos, WorldRngSeeder};
use block::{AtomicBlockId, BlockId};
use rand::IsaacRng;
use world::Generator as WorldGenerator;

pub trait Structure where Self: Send + Sync {
    fn generate<'a>(&self, &'a mut GeneratingChunk<'a>, &mut IsaacRng, &WorldGenerator);
}

pub struct StructureList(Vec<(Box<Structure>, BlockPos, [Range<i32>; 3])>, );

impl StructureList {
    pub fn push(&mut self, structure: Box<Structure>, pos: BlockPos, bounds: [[i32; 2]; 3]) {
        let cs = CHUNK_SIZE as i32;
        let mut chunk_range = [0..0, 0..0, 0..0];
        for i in 0..3 {
            let min = (pos[i] - bounds[i][0]).div_floor(&cs);
            let max = (pos[i] + bounds[i][1]).div_floor(&cs);
            chunk_range[i] = min..(max + 1)
        }
        self.0.push((structure, pos, chunk_range));
    }
}

pub trait StructureFinder where Self: Send + Sync {
    fn push_structures(&self,
                       chunk: ChunkPos,
                       seeder: &mut IsaacRng,
                       parent: &WorldGenerator,
                       out: &mut StructureList);
    fn max_bounds(&self) -> [[i32; 2]; 3];
}

pub struct GeneratingChunk<'a> {
    chunk: &'a mut ChunkArray<AtomicBlockId>,
    struct_pos: [i32; 3],
}

impl<'a> GeneratingChunk<'a> {
    pub fn pos_in_chunk(&self, pos: [i32; 3]) -> Option<[usize; 3]> {
        let pos = vec3_add(pos, self.struct_pos);
        if pos.iter().all(|x| *x >= 0 && *x < CHUNK_SIZE as i32) {
            Some([
                pos[0] as usize,
                pos[1] as usize,
                pos[2] as usize,
            ])
        } else {
            None
        }
    }

    pub fn pos(&self) -> [i32; 3] {
        vec3_scale(self.struct_pos, -1)
    }

    pub fn set_block(&mut self, pos: [i32; 3], block: BlockId) -> bool {
        if let Some(pos) = self.pos_in_chunk(pos) {
            self.chunk[pos] = AtomicBlockId::new(block);
            true
        } else {
            false
        }
    }

    pub fn get_block(&mut self, pos: [i32; 3]) -> Option<BlockId> {
        if let Some(pos) = self.pos_in_chunk(pos) {
            Some(self.chunk[pos].load())
        } else {
            None
        }
    }

    pub fn blocks(&mut self) -> &mut ChunkArray<AtomicBlockId> {
        &mut self.chunk
    }
}

pub struct CombinedStructureGenerator {
    finders: Vec<Box<StructureFinder>>,
    cached: CHashMap<ChunkPos, StructureList>,
    max_bounds: [Range<i32>; 3],
    seeder: WorldRngSeeder,
}

impl CombinedStructureGenerator {
    pub fn new(finders: Vec<Box<StructureFinder>>, seeder: WorldRngSeeder) -> Self {
        let mut bounds = [[0; 2]; 3];
        for fb in finders.iter().map(|f| f.max_bounds()) {
            for i in 0..3 {
                for j in 0..2 {
                    bounds[i][j] = cmp::max(bounds[i][j], fb[i][j])
                }
            }
        }
        let mut chunk_range = [0..0, 0..0, 0..0];
        for i in 0..3 {
            let min = -(bounds[i][0] + CHUNK_SIZE as i32 - 1) / CHUNK_SIZE as i32;
            let max = (bounds[i][1] + CHUNK_SIZE as i32 - 1) / CHUNK_SIZE as i32;
            chunk_range[i] = min..(max + 1);
        }
        CombinedStructureGenerator {
            finders: finders,
            cached: CHashMap::new(),
            max_bounds: chunk_range,
            seeder: seeder,
        }
    }

    pub fn reseed(&mut self,s:&WorldRngSeeder){
        self.seeder=s.clone();
        self.cached.clear();
    }

    pub fn generate_chunk(&self, pos: ChunkPos, chunk: &mut ChunkArray<AtomicBlockId>,parent:&WorldGenerator) {
        let cs = CHUNK_SIZE as i32;
        let mut rand = self.seeder.make_gen(pos[0], pos[1], pos[2]);
        for x in self.max_bounds[0].clone() {
            for y in self.max_bounds[1].clone() {
                for z in self.max_bounds[2].clone() {
                    self.with_chunk(ChunkPos([x + pos[0], y + pos[1], z + pos[2]]), |structures| {
                        for s in structures.0.iter()
                            .filter(|s| {
                                s.2[0].contains(pos[0])
                                    && s.2[1].contains(pos[1])
                                    && s.2[2].contains(pos[2])
                            }) {
                            let rel_struct_pos = vec3_sub((s.1).0, vec3_scale(pos.0, cs));
                            let mut gen_chunk = GeneratingChunk {
                                chunk: chunk,
                                struct_pos: rel_struct_pos,
                            };
                            s.0.generate(&mut gen_chunk, &mut rand,parent);
                        }
                    },parent);
                }
            }
        }
    }

    fn with_chunk<F: FnOnce(&StructureList)>(&self, pos: ChunkPos, f: F,gen:&WorldGenerator) {
        self.cached.alter(pos, |opt_list| {
            if let Some(list) = opt_list {
                f(&list);
                Some(list)
            } else {
                let list = self.find_structures(pos,gen);
                f(&list);
                Some(list)
            }
        });
    }

    fn find_structures(&self, pos: ChunkPos,gen:&WorldGenerator) -> StructureList {
        let mut ret = StructureList(Vec::new());
        let mut rand = self.seeder.make_gen(pos[0], pos[1], pos[2]);
        for finder in self.finders.iter() {
            finder.push_structures(pos, &mut rand, gen, &mut ret);
        }
        ret
    }
}