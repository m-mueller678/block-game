use std::collections::hash_map::*;
use std::sync::Arc;
use std::sync::atomic::{Ordering};
use num::Integer;
use block::{BlockId, BlockRegistry, LightType};
use geometry::{Direction};
use geometry::ray::{Ray, BlockIntersection};

mod inserter;
mod position;
mod lighting;
mod atomic_light;
mod chunk;

pub use self::position::*;
pub use self::inserter::Inserter;
pub use self::chunk::*;

use self::lighting::*;

struct ChunkColumn {
    chunks: [Vec<Option<Chunk>>; 2],
}

impl ChunkColumn {
    fn new() -> Self {
        ChunkColumn {
            chunks: [Vec::new(), Vec::new()],
        }
    }
    fn get(&self, y: i32) -> Option<&Chunk> {
        let pos = y >= 0;
        let index = if pos {
            y as usize
        } else {
            (-y - 1) as usize
        };
        self.chunks[pos as usize].get(index).map(|o| o.as_ref()).unwrap_or(None)
    }
    fn insert(&mut self, y: i32, chunk: Chunk) -> &mut Chunk {
        let pos = y >= 0;
        let index = if pos {
            y as usize
        } else {
            (-y - 1) as usize
        };
        let vec = &mut self.chunks[pos as usize];
        if index >= vec.len() {
            let new_len = index + 1;
            let len_dif = new_len - vec.len();
            vec.reserve(len_dif);
            for _ in 0..(len_dif - 1) {
                vec.push(None);
            }
            vec.push(Some(chunk));
            vec[index].as_mut().unwrap()
        } else {
            assert!(vec[index].is_none());
            vec[index] = Some(chunk);
            vec[index].as_mut().unwrap()
        }
    }
    fn remove(&mut self, y: i32) -> Option<Chunk> {
        let pos = y >= 0;
        let index = if pos { y as usize } else { -y as usize - 1 };
        self.chunks[pos as usize].get_mut(index).and_then(|ref mut opt| opt.take())
    }
    fn empty(&self) -> bool {
        self.chunks.iter().all(|v| v.iter().all(|o| o.is_none()))
    }
}

pub struct ChunkMap {
    columns: HashMap<[i32; 2], ChunkColumn>,
    blocks: Arc<BlockRegistry>,
}

impl ChunkMap {
    pub fn new(blocks: Arc<BlockRegistry>) -> Self {
        ChunkMap {
            columns: HashMap::new(),
            blocks: blocks,
        }
    }
    pub fn remove_chunk(&mut self, pos: &ChunkPos) -> Option<Chunk> {
        if let Entry::Occupied(mut col) = self.columns.entry([pos[0], pos[2]]) {
            let ret = col.get_mut().remove(pos[1]);
            if col.get().empty() {
                col.remove();
            }
            ret
        } else {
            None
        }
    }
    pub fn set_block(&self, pos: &BlockPos, block: BlockId) -> Result<(), ()> {
        let chunk_pos = Self::chunk_at(pos);
        if let Some(chunk) = self.borrow_chunk(&chunk_pos) {
            let before;
            {
                before = chunk.data[*pos].load();
                chunk.data[*pos].store(block);
            }
            match (*self.blocks.light_type(before), *self.blocks.light_type(block)) {
                (LightType::Transparent, LightType::Transparent)
                | (LightType::Opaque, LightType::Opaque) => {}
                (LightType::Source(s1), LightType::Source(s2)) => {
                    if s2 > s1 {
                        increase_light(
                            &mut self.artificial_lightmap(chunk_pos),
                            UpdateQueue::single(s2, pos.clone(), None));
                    } else if s2 < s1 {
                        remove_and_relight(&mut self.artificial_lightmap(chunk_pos), &[pos.clone()]);
                    }
                }
                (LightType::Source(_), LightType::Transparent) => {
                    remove_and_relight(&mut self.artificial_lightmap(chunk_pos), &[pos.clone()]);
                }
                (LightType::Transparent, LightType::Source(s)) => {
                    increase_light(
                        &mut self.artificial_lightmap(chunk_pos),
                        UpdateQueue::single(s, pos.clone(), None));
                }
                (LightType::Opaque, _) => {
                    relight(&mut self.artificial_lightmap(chunk_pos.clone()), pos);
                    relight(&mut self.natural_lightmap(chunk_pos), pos);
                }
                (_, LightType::Opaque) => {
                    remove_and_relight(&mut self.artificial_lightmap(chunk_pos.clone()), &[pos.clone()]);
                    remove_and_relight(&mut self.natural_lightmap(chunk_pos), &[pos.clone()]);
                }
            }
            chunk.update_render.store(true, Ordering::Release);
            if self.blocks.is_opaque_draw(before) ^ self.blocks.is_opaque_draw(block) {
                self.update_adjacent_chunks(pos);
            }
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn reset_chunk_updated(&self, pos: &ChunkPos) -> bool {
        self.borrow_chunk(pos).map(|c| c.update_render.swap(false, Ordering::Acquire)).unwrap_or(false)
    }
    pub fn chunk_loaded(&self, pos: &ChunkPos) -> bool {
        self.borrow_chunk(pos).is_some()
    }
    pub fn chunk_at(pos: &BlockPos) -> ChunkPos {
        use num::Integer;
        ChunkPos([
            pos[0].div_floor(&(CHUNK_SIZE as i32)),
            pos[1].div_floor(&(CHUNK_SIZE as i32)),
            pos[2].div_floor(&(CHUNK_SIZE as i32)),
        ])
    }
    pub fn lock_chunk(&self, pos: &ChunkPos) -> Option<ChunkReader> {
        self.borrow_chunk(pos).map(|x| ChunkReader::new(x))
    }
    pub fn block_ray_trace(&self, start: [f32; 3], direction: [f32; 3], range: f32) -> Option<BlockIntersection> {
        for intersect in Ray::new(start, direction).blocks() {
            let sq_dist: f32 = intersect.block.0.iter()
                .map(|x| *x as f32 + 0.5)
                .zip(start.iter()).map(|x| x.1 - x.0)
                .map(|x| x * x).sum();
            if sq_dist > range * range {
                return None;
            }
            if let Some(id) = self.get_block(&intersect.block) {
                if self.blocks.is_opaque_draw(id) {
                    return Some(intersect)
                }
            }
        }
        unreachable!()// ray block iterator is infinite
    }
    pub fn get_block(&self, pos: &BlockPos) -> Option<BlockId> {
        self.borrow_chunk(&Self::chunk_at(pos)).map(|c| c.data[*pos].load())
    }
    pub fn natural_light(&self, pos: &BlockPos) -> Option<(u8, Option<Direction>)> {
        if let Some(chunk) = self.borrow_chunk(&Self::chunk_at(pos)) {
            let light = &chunk.natural_light[*pos];
            Some((light.level(), light.direction()))
        } else {
            None
        }
    }
    pub fn artificial_light(&self, pos: &BlockPos) -> Option<(u8, Option<Direction>)> {
        if let Some(chunk) = self.borrow_chunk(&Self::chunk_at(pos)) {
            let light = &chunk.artificial_light[*pos];
            Some((light.level(), light.direction()))
        } else {
            None
        }
    }
    pub fn blocks(&self) -> &BlockRegistry {
        &*self.blocks
    }
    fn artificial_lightmap(&self, p: ChunkPos) -> ArtificialLightMap {
        ArtificialLightMap::new(&self, ChunkCache::new(p, &self).unwrap())
    }
    fn natural_lightmap(&self, p: ChunkPos) -> NaturalLightMap {
        NaturalLightMap::new(&self, ChunkCache::new(p, &self).unwrap())
    }
    fn trigger_chunk_face_brightness(&self,
                                     pos: &ChunkPos,
                                     face: Direction,
                                     artificial_updates: &mut UpdateQueue,
                                     natural_updates: &mut UpdateQueue) {
        let (positive, d1, d2, face_direction) = match face {
            Direction::PosX => (true, 1, 2, 0),
            Direction::NegX => (false, 1, 2, 0),
            Direction::PosY => (true, 0, 2, 1),
            Direction::NegY => (false, 0, 2, 1),
            Direction::PosZ => (true, 0, 1, 2),
            Direction::NegZ => (false, 0, 1, 2),
        };

        let mut brightness = [[(0, 0); CHUNK_SIZE]; CHUNK_SIZE];
        let chunk = self.borrow_chunk(pos).unwrap();
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut block_pos = [0, 0, 0];
                block_pos[d1] = i;
                block_pos[d2] = j;
                block_pos[face_direction] = if positive { CHUNK_SIZE - 1 } else { 0 };
                brightness[i][j].0 = chunk.artificial_light[block_pos].level();
                brightness[i][j].1 = chunk.natural_light[block_pos].level();
            }
        }
        let chunk_size = CHUNK_SIZE as i32;
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                let mut block_pos = BlockPos([pos[0] * chunk_size, pos[1] * chunk_size, pos[2] * chunk_size]);
                block_pos.0[d1] += i as i32;
                block_pos.0[d2] += j as i32;
                block_pos.0[face_direction] += if positive { CHUNK_SIZE as i32 } else { -1 };

                if brightness[i][j].0 > 1 {
                    artificial_updates.push(brightness[i][j].0 - 1, block_pos.clone(), Some(face));
                }
                if brightness[i][j].1 > 1 {
                    if face == Direction::NegY && brightness[i][j].1 == MAX_NATURAL_LIGHT {
                        natural_updates.push(MAX_NATURAL_LIGHT, block_pos, Some(face));
                    } else {
                        natural_updates.push(brightness[i][j].1 - 1, block_pos, Some(face));
                    }
                }
            }
        }
    }
    fn update_adjacent_chunks(&self, block_pos: &BlockPos) {
        let cs = CHUNK_SIZE as i32;
        let chunk_pos = Self::chunk_at(block_pos);
        let (x, y, z) = (chunk_pos[0], chunk_pos[1], chunk_pos[2]);
        if block_pos[0].mod_floor(&cs) == 0 { self.update_render(&ChunkPos([x - 1, y, z])) }
        if block_pos[1].mod_floor(&cs) == 0 { self.update_render(&ChunkPos([x, y - 1, z])) }
        if block_pos[2].mod_floor(&cs) == 0 { self.update_render(&ChunkPos([x, y, z - 1])) }
        if block_pos[0].mod_floor(&cs) == cs - 1 { self.update_render(&ChunkPos([x + 1, y, z])) }
        if block_pos[1].mod_floor(&cs) == cs - 1 { self.update_render(&ChunkPos([x, y + 1, z])) }
        if block_pos[2].mod_floor(&cs) == cs - 1 { self.update_render(&ChunkPos([x, y, z + 1])) }
    }
    fn update_render(&self, pos: &ChunkPos) {
        if let Some(chunk) = self.borrow_chunk(pos) {
            chunk.update_render.store(true, Ordering::Release)
        }
    }
    fn borrow_chunk(&self, p: &ChunkPos) -> Option<&Chunk> {
        self.columns.get(&[p[0], p[2]]).and_then(|col| col.get(p[1]))
    }
}

pub fn chunk_at(pos: &BlockPos) -> ChunkPos {
    ChunkMap::chunk_at(pos)
}