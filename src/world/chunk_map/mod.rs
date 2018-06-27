use chashmap::{CHashMap, ReadGuard};
use std::sync::{Arc, Mutex};
use num::Integer;
use block::{BlockId, LightType};
use geometry::{Direction, ALL_DIRECTIONS};
use module::GameData;
use logging::*;
use graphics::ChunkUpdateSender;

mod position;
mod lighting;
mod atomic_light;
mod chunk;
mod chunk_cache;

pub use self::position::*;
pub use self::chunk::*;
pub use self::chunk_cache::ChunkCache;

use self::lighting::*;

pub struct ChunkMap {
    chunks: CHashMap<[i32; 3], Arc<Chunk>>,
    game_data: GameData,
    logger: Logger,
    graphics_update_sender: Mutex<ChunkUpdateSender>,
}

impl ChunkMap {
    pub fn new(game_data: GameData, update: ChunkUpdateSender) -> Self {
        ChunkMap {
            chunks: CHashMap::new(),
            game_data: game_data,
            logger: root_logger().clone(),
            graphics_update_sender: Mutex::new(update),
        }
    }
    pub fn remove_chunk(&self, pos: ChunkPos) -> Option<Arc<Chunk>> {
        self.chunks.remove(&[pos[0], pos[1], pos[2]])
    }
    pub fn set_block(&self, pos: BlockPos, block: BlockId) -> Result<(), ()> {
        let chunk_pos = Self::chunk_at(pos);
        if let Some(chunk) = self.borrow_chunk(chunk_pos) {
            let before;
            {
                before = chunk.data[pos].load();
                chunk.data[pos].store(block);
            }
            match (*self.game_data.blocks().light_type(before),
                   *self.game_data.blocks().light_type(block)) {
                (LightType::Transparent, LightType::Transparent) |
                (LightType::Opaque, LightType::Opaque) => {}
                (LightType::Source(s1), LightType::Source(s2)) => {
                    if s2 > s1 {
                        increase_light(&mut self.artificial_lightmap(chunk_pos),
                                       UpdateQueue::single(s2, pos, None));
                    } else if s2 < s1 {
                        remove_and_relight(&mut self.artificial_lightmap(chunk_pos), &[pos]);
                    }
                }
                (LightType::Source(_), LightType::Transparent) => {
                    remove_and_relight(&mut self.artificial_lightmap(chunk_pos), &[pos]);
                }
                (LightType::Transparent, LightType::Source(s)) => {
                    increase_light(&mut self.artificial_lightmap(chunk_pos),
                                   UpdateQueue::single(s, pos, None));
                }
                (LightType::Opaque, _) => {
                    relight(&mut self.artificial_lightmap(chunk_pos), pos);
                    relight(&mut self.natural_lightmap(chunk_pos), pos);
                }
                (_, LightType::Opaque) => {
                    remove_and_relight(&mut self.artificial_lightmap(chunk_pos), &[pos]);
                    remove_and_relight(&mut self.natural_lightmap(chunk_pos), &[pos]);
                }
            }
            self.update_render(chunk_pos);
            if self.game_data.blocks().is_opaque_draw(before) ^
                self.game_data.blocks().is_opaque_draw(block) {
                self.update_adjacent_chunks(pos);
            }
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.borrow_chunk(pos).is_some()
    }
    pub fn chunk_at(pos: BlockPos) -> ChunkPos {
        use num::Integer;
        ChunkPos([pos[0].div_floor(&(CHUNK_SIZE as i32)),
            pos[1].div_floor(&(CHUNK_SIZE as i32)),
            pos[2].div_floor(&(CHUNK_SIZE as i32))])
    }

    pub fn get_block(&self, pos: BlockPos) -> Option<BlockId> {
        self.borrow_chunk(Self::chunk_at(pos))
            .map(|c| c.data[pos].load())
    }
    pub fn natural_light(&self, pos: BlockPos) -> Option<(u8, Option<Direction>)> {
        if let Some(chunk) = self.borrow_chunk(Self::chunk_at(pos)) {
            let light = &chunk.natural_light[pos];
            Some((light.level(), light.direction()))
        } else {
            None
        }
    }
    pub fn artificial_light(&self, pos: BlockPos) -> Option<(u8, Option<Direction>)> {
        if let Some(chunk) = self.borrow_chunk(Self::chunk_at(pos)) {
            let light = &chunk.artificial_light[pos];
            Some((light.level(), light.direction()))
        } else {
            None
        }
    }

    pub fn insert_chunk(&self, insert_pos: ChunkPos, chunk: Arc<Chunk>, light_sources: &[(BlockPos, u8)]) {
        let mut sources_to_trigger = UpdateQueue::new();
        self.chunks.insert([insert_pos[0], insert_pos[1], insert_pos[2]], chunk);
        for source in light_sources {
            sources_to_trigger.push(source.1, source.0, None);
        }

        let cs = CHUNK_SIZE as i32;
        let mut sky_light = UpdateQueue::new();
        if !self.chunk_loaded(insert_pos.facing(Direction::PosY)) {
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let abs_pos = BlockPos(
                        [
                            cs * insert_pos[0] + x as i32,
                            cs * insert_pos[1] + cs - 1,
                            cs * insert_pos[2] + z as i32,
                        ],
                    );
                    sky_light.push(MAX_NATURAL_LIGHT, abs_pos, Some(Direction::NegY));
                }
            }
        }
        for face in &ALL_DIRECTIONS {
            let facing = insert_pos.facing(*face);
            if self.chunk_loaded(facing) {
                self.trigger_chunk_face_brightness(
                    facing,
                    face.invert(),
                    &mut sources_to_trigger,
                    &mut sky_light,
                );
                self.update_render(facing);
            }
        }
        increase_light(
            &mut self.artificial_lightmap(insert_pos),
            sources_to_trigger,
        );
        increase_light(&mut self.natural_lightmap(insert_pos), sky_light);

        //block natural light in chunk below
        if self.chunk_loaded(insert_pos.facing(Direction::NegY)) {
            let mut relight = RelightData::new();
            let mut lm = self.natural_lightmap(insert_pos);
            let inserted_cache = ChunkCache::new(insert_pos, &self).unwrap();
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let abs_pos = BlockPos(
                        [
                            insert_pos[0] * cs + x as i32,
                            insert_pos[1] * cs - 1,
                            insert_pos[2] * cs + z as i32,
                        ],
                    );
                    if inserted_cache.chunk().natural_light[[x, 0, z]].level() != MAX_NATURAL_LIGHT {
                        remove_light_rec(&mut lm, abs_pos, Direction::NegY, &mut relight);
                    }
                }
            }
            increase_light(
                &mut self.natural_lightmap(insert_pos.facing(Direction::NegY)),
                relight.build_queue(&mut lm),
            );
        }
    }
    fn artificial_lightmap(&self, p: ChunkPos) -> ArtificialLightMap {
        ArtificialLightMap::new(self, ChunkCache::new(p, self).unwrap())
    }
    fn natural_lightmap(&self, p: ChunkPos) -> NaturalLightMap {
        NaturalLightMap::new(self, ChunkCache::new(p, self).unwrap())
    }
    fn trigger_chunk_face_brightness(&self,
                                     pos: ChunkPos,
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
        let chunk = match self.borrow_chunk(pos) {
            Some(chunk) => chunk,
            None => {
                error!(self.logger,
                       "chunk at {:?} disappeared before face brightness update",
                       pos);
                return;
            }
        };
        for (i, brightness) in brightness.iter_mut().enumerate() {
            for (j, brightness) in brightness.iter_mut().enumerate() {
                let mut block_pos = [0, 0, 0];
                block_pos[d1] = i;
                block_pos[d2] = j;
                block_pos[face_direction] = if positive { CHUNK_SIZE - 1 } else { 0 };
                brightness.0 = chunk.artificial_light[block_pos].level();
                brightness.1 = chunk.natural_light[block_pos].level();
            }
        }
        let chunk_size = CHUNK_SIZE as i32;
        for (i, brightness) in brightness.iter_mut().enumerate() {
            for (j, brightness) in brightness.iter_mut().enumerate() {
                let mut block_pos = BlockPos([pos[0] * chunk_size,
                    pos[1] * chunk_size,
                    pos[2] * chunk_size]);
                block_pos.0[d1] += i as i32;
                block_pos.0[d2] += j as i32;
                block_pos.0[face_direction] += if positive { CHUNK_SIZE as i32 } else { -1 };

                if brightness.0 > 1 {
                    artificial_updates.push(brightness.0 - 1, block_pos, Some(face));
                }
                if brightness.1 > 1 {
                    if face == Direction::NegY && brightness.1 == MAX_NATURAL_LIGHT {
                        natural_updates.push(MAX_NATURAL_LIGHT, block_pos, Some(face));
                    } else {
                        natural_updates.push(brightness.1 - 1, block_pos, Some(face));
                    }
                }
            }
        }
    }
    fn update_adjacent_chunks(&self, block_pos: BlockPos) {
        let cs = CHUNK_SIZE as i32;
        let chunk_pos = Self::chunk_at(block_pos);
        let (x, y, z) = (chunk_pos[0], chunk_pos[1], chunk_pos[2]);
        if block_pos[0].mod_floor(&cs) == 0 {
            self.update_render(ChunkPos([x - 1, y, z]))
        }
        if block_pos[1].mod_floor(&cs) == 0 {
            self.update_render(ChunkPos([x, y - 1, z]))
        }
        if block_pos[2].mod_floor(&cs) == 0 {
            self.update_render(ChunkPos([x, y, z - 1]))
        }
        if block_pos[0].mod_floor(&cs) == cs - 1 {
            self.update_render(ChunkPos([x + 1, y, z]))
        }
        if block_pos[1].mod_floor(&cs) == cs - 1 {
            self.update_render(ChunkPos([x, y + 1, z]))
        }
        if block_pos[2].mod_floor(&cs) == cs - 1 {
            self.update_render(ChunkPos([x, y, z + 1]))
        }
    }
    fn update_render(&self, pos: ChunkPos) {
        if let Some(chunk) = self.chunks.get(&*pos) {
            self.graphics_update_sender.lock().unwrap().send(pos, &*chunk);
        }
    }
    fn borrow_chunk(&self, p: ChunkPos) -> Option<ReadGuard<[i32; 3], Arc<Chunk>>> {
        self.chunks.get(&[p[0], p[1], p[2]])
    }
}

pub fn chunk_at(pos: BlockPos) -> ChunkPos {
    ChunkMap::chunk_at(pos)
}
