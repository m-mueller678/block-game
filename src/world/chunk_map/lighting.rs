use geometry::{Direction, ALL_DIRECTIONS};
use world::BlockPos;
use super::{ChunkMap, ChunkCache, chunk_at};
use block::LightType;

pub const MAX_NATURAL_LIGHT: u8 = 5;

pub type Light = (u8, Option<Direction>);

pub struct UpdateQueue {
    levels: Vec<Vec<(BlockPos, Option<Direction>)>>,
}

pub struct RelightData {
    //position and brightness of source
    sources: Vec<(BlockPos, u8)>,
    //position of brighter block and direction brightness should be updated
    brighter: Vec<(BlockPos, Direction)>,
}

impl RelightData {
    pub fn new() -> Self {
        RelightData {
            sources: Vec::new(),
            brighter: Vec::new(),
        }
    }
    pub fn build_queue<LM: LightMap>(&self, lm: &mut LM) -> UpdateQueue {
        let mut queue = UpdateQueue::new();
        for s in &self.sources {
            queue.push(s.1, s.0, None);
        }
        for b in &self.brighter {
            let own_light = lm.get_light(b.0);
            if own_light.0 > 1 {
                let light_out = lm.compute_light_to(b.1, own_light.0);
                queue.push(light_out, b.0.facing(b.1), Some(b.1));
            }
        }
        queue
    }
}

impl UpdateQueue {
    pub fn single(level: u8, pos: BlockPos, direction: Option<Direction>) -> Self {
        let mut uq = Self::new();
        uq.push(level, pos, direction);
        uq
    }
    pub fn new() -> Self {
        UpdateQueue { levels: Vec::new() }
    }

    pub fn push(&mut self, level: u8, pos: BlockPos, direction: Option<Direction>) {
        assert!(level >= 1);
        let level_index = level as usize - 1;
        while self.levels.len() <= level_index {
            self.levels.push(Vec::new())
        }
        self.levels[level_index].push((pos, direction))
    }
}

pub trait LightMap {
    fn is_opaque(&mut self, pos: BlockPos) -> bool;
    fn get_light(&mut self, pos: BlockPos) -> Light;
    fn set_light(&mut self, pos: BlockPos, Light);
    // level > 1
    fn compute_light_to(&mut self, direction: Direction, level: u8) -> u8;
    fn internal_light(&mut self, block: BlockPos) -> u8;
}

pub fn remove_light_rec<LM: LightMap>(
    lm: &mut LM,
    pos: BlockPos,
    light_direction: Direction,
    relight_data: &mut RelightData,
) {
    let light = lm.get_light(pos);
    if light.1 == Some(light_direction) {
        let internal_light = lm.internal_light(pos);
        if internal_light < light.0 {
            if internal_light > 1 {
                lm.set_light(pos, (0, None));
                relight_data.sources.push((pos, internal_light));
            } else {
                lm.set_light(pos, (internal_light, None));
            }
            for d in &ALL_DIRECTIONS {
                remove_light_rec(lm, pos.facing(*d), *d, relight_data);
            }
        }
    } else if light.0 > 1 {
        relight_data.brighter.push((pos, light_direction.invert()));
    }
}


pub fn relight<LM: LightMap>(lm: &mut LM, pos: BlockPos) {
    let mut updates = UpdateQueue::new();
    let internal_light = lm.internal_light(pos);
    if internal_light > 0 {
        updates.push(internal_light, pos, None);
    }
    for d in &ALL_DIRECTIONS {
        let adjacent_light = lm.get_light(pos.facing(d.invert())).0;
        if adjacent_light > 1 {
            updates.push(lm.compute_light_to(*d, adjacent_light), pos, Some(*d));
        }
    }
    increase_light(lm, updates);
}

pub fn remove_and_relight<LM: LightMap>(lm: &mut LM, positions: &[BlockPos]) {
    let mut update_brightness = RelightData::new();
    for &pos in positions {
        let internal_light = lm.internal_light(pos);
        if internal_light > 1 {
            lm.set_light(pos, (0, None));
            update_brightness.sources.push((pos, internal_light));
        } else {
            lm.set_light(pos, (internal_light, None));
        }
        for d in &ALL_DIRECTIONS {
            remove_light_rec(lm, pos.facing(*d), *d, &mut update_brightness);
        }
    }
    let queue = update_brightness.build_queue(lm);
    increase_light(lm, queue);
}

pub fn increase_light<LM: LightMap>(lm: &mut LM, mut to_update: UpdateQueue) {
    while !to_update.levels.is_empty() {
        let update_len = to_update.levels.len();
        let level = update_len as u8;
        if level == 0 {
            continue;
        }
        while let Some(current_pos) = to_update.levels[update_len - 1].pop() {
            if lm.is_opaque(current_pos.0) {
                continue;
            }
            let light = lm.get_light(current_pos.0);
            if level > light.0 {
                lm.set_light(current_pos.0, (level, current_pos.1));
                if level > 1 {
                    for d in &ALL_DIRECTIONS {
                        let adjacent_level = lm.compute_light_to(*d, level);
                        assert!(adjacent_level <= level);
                        if adjacent_level > 0 {
                            to_update.push(adjacent_level, current_pos.0.facing(*d), Some(*d))
                        }
                    }
                }
            }
        }
        to_update.levels.pop();
    }
}

pub struct ArtificialLightMap<'a> {
    world: &'a ChunkMap,
    cache: ChunkCache<'a>,
}

impl<'a> ArtificialLightMap<'a> {
    pub fn new(world: &'a ChunkMap, cache: ChunkCache<'a>) -> Self {
        ArtificialLightMap {
            world: world,
            cache: cache,
        }
    }
}

impl<'a> LightMap for ArtificialLightMap<'a> {
    fn is_opaque(&mut self, pos: BlockPos) -> bool {
        if self.cache
            .load(ChunkMap::chunk_at(pos), self.world)
            .is_err()
        {
            true
        } else {
            self.world
                .game_data
                .blocks()
                .light_type(self.cache.chunk.data[pos].load())
                .is_opaque()
        }
    }

    fn get_light(&mut self, pos: BlockPos) -> Light {
        if self.cache
            .load(ChunkMap::chunk_at(pos), self.world)
            .is_err()
        {
            (0, None)
        } else {
            let atomic_light = &self.cache.chunk.artificial_light[pos];
            (atomic_light.level(), atomic_light.direction())
        }
    }

    fn set_light(&mut self, pos: BlockPos, light: Light) {
        self.cache.chunk.artificial_light[pos].set(light.0, light.1);
        self.cache.set_update(self.world);
        self.world.update_adjacent_chunks(pos);
    }
    fn compute_light_to(&mut self, _: Direction, level: u8) -> u8 {
        level - 1
    }
    fn internal_light(&mut self, pos: BlockPos) -> u8 {
        if self.cache.load(chunk_at(pos), self.world).is_err() {
            0
        } else {
            match *self.world.game_data.blocks().light_type(
                self.cache.chunk.data[pos]
                    .load(),
            ) {
                LightType::Source(s) => s,
                LightType::Opaque | LightType::Transparent => 0,
            }
        }
    }
}

pub struct NaturalLightMap<'a> {
    world: &'a ChunkMap,
    cache: ChunkCache<'a>,
}

impl<'a> NaturalLightMap<'a> {
    pub fn new(world: &'a ChunkMap, cache: ChunkCache<'a>) -> Self {
        NaturalLightMap {
            world: world,
            cache: cache,
        }
    }
}

impl<'a> LightMap for NaturalLightMap<'a> {
    fn is_opaque(&mut self, pos: BlockPos) -> bool {
        if self.cache.load(chunk_at(pos), self.world).is_err() {
            true
        } else {
            self.world
                .game_data
                .blocks()
                .light_type(self.cache.chunk.data[pos].load())
                .is_opaque()
        }
    }

    fn get_light(&mut self, pos: BlockPos) -> Light {
        if self.cache.load(chunk_at(pos), self.world).is_err() {
            (0, None)
        } else {
            let atomic_light = &self.cache.chunk.natural_light[pos];
            (atomic_light.level(), atomic_light.direction())
        }
    }

    fn set_light(&mut self, pos: BlockPos, light: Light) {
        self.cache.chunk.natural_light[pos].set(light.0, light.1);
        self.cache.set_update(self.world);
        self.world.update_adjacent_chunks(pos);
    }

    fn compute_light_to(&mut self, d: Direction, level: u8) -> u8 {
        if level == MAX_NATURAL_LIGHT && d == Direction::NegY {
            MAX_NATURAL_LIGHT
        } else {
            level - 1
        }
    }

    fn internal_light(&mut self, _: BlockPos) -> u8 {
        0
    }
}
