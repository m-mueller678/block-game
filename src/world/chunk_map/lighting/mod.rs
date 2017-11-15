use std::collections::HashSet;
use std::mem;
use geometry::{ALL_DIRECTIONS, Direction};
use world::*;
use block::LightType;
use super::atomic_light::LightState;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LightDirection {
    SelfLit,
    Directed(Direction),
}

pub type Light = (u8, LightDirection);

const MAX_LIGHT: u8 = 16;

mod light_increase;
mod light_decrease;

use self::light_increase::LightIncrease;
use self::light_decrease::LightDecrease;

pub struct LightUpdater {
    add: Mutex<LightIncrease>,
    remove: Mutex<LightDecrease>,
    refill: Mutex<Vec<BlockPos>>,
}

impl LightUpdater {
    pub fn new() -> Self {
        LightUpdater {
            add: Mutex::new(LightIncrease::new()),
            remove: Mutex::new(LightDecrease::new()),
            refill: Default::default(),
        }
    }

    /// spread artificial light from chunk_pos to the one adjacent to it
    pub fn trigger_artificial_chunk_face(&self,
                                         chunk_light: &ChunkArray<LightState>,
                                         chunk_pos: ChunkPos,
                                         direction: Direction) {
        let base_pos = chunk_pos.first_block();
        let axis = direction.axis();
        let iter_axis_1 = if axis == 2 { 0 } else { axis + 1 };
        let iter_axis_2 = if axis == 0 { 2 } else { axis - 1 };
        let mut offset_pos = [0; 3];
        if direction.positive() {
            offset_pos[axis] = CHUNK_SIZE - 1;
        }
        let direction_offset = if direction.positive() { 1 } else { -1 };
        let mut add_lock = self.add.lock().unwrap();
        loop {
            let light = &chunk_light[offset_pos];
            let level = light.level();
            if level > 1 {
                let mut global_pos = [base_pos[0] + offset_pos[0] as i32,
                                      base_pos[1] + offset_pos[1] as i32,
                                      base_pos[2] + offset_pos[2] as i32];
                global_pos[axis] += direction_offset;
                add_lock.push(BlockPos(global_pos),
                              (level - 1, LightDirection::Directed(direction)));
            }
            offset_pos[iter_axis_1] += 1;
            if offset_pos[iter_axis_1] >= CHUNK_SIZE {
                offset_pos[iter_axis_1] = 0;
                offset_pos[iter_axis_2] += 1;
                if offset_pos[iter_axis_2] >= CHUNK_SIZE {
                    break;
                }
            }
        }
    }

    pub fn block_light_changed(&self, old_light: Light, light_type: &LightType, pos: BlockPos) {
        let old_level = old_light.0;
        match *light_type {
            LightType::Opaque => {
                if old_level != 0 {
                    self.remove.lock().unwrap().push(pos, old_level, None);
                }
            }
            LightType::Source(b) => {
                if b < old_level {
                    if old_light.1 == LightDirection::SelfLit {
                        self.remove
                            .lock()
                            .unwrap()
                            .push(pos, old_level, Some(LightDirection::SelfLit));
                        self.refill.lock().unwrap().push(pos);
                        self.add
                            .lock()
                            .unwrap()
                            .push(pos, (b, LightDirection::SelfLit));
                    }
                } else if b > old_level {
                    self.add
                        .lock()
                        .unwrap()
                        .push(pos, (b, LightDirection::SelfLit));
                }
            }
            LightType::Transparent => {
                if old_level == 0 {
                    self.refill.lock().unwrap().push(pos);
                } else if old_light.1 == LightDirection::SelfLit {
                    self.remove
                        .lock()
                        .unwrap()
                        .push(pos, old_level, Some(LightDirection::SelfLit));
                }
            }
        }
    }

    pub fn apply(&self, world: &ChunkMap) {
        let mut changed = HashSet::new();
        let mut remove = self.remove.lock().unwrap();
        let mut refill = self.refill.lock().unwrap();
        let mut add = self.add.lock().unwrap();
        let mut chunk_cache = ChunkCache::new();

        //remove light
        let mut brighter = Vec::with_capacity(512);
        while let Some((pos, direction_filter)) = remove.pop() {
            //load light
            let containing = pos.containing_chunk();
            changed.insert(containing);
            if chunk_cache.load(containing, world).is_err() {
                continue;
            }
            let light = &chunk_cache.chunk().artificial_light[pos];
            let old_direction = light.direction();
            let old_level = light.level();
            if old_level == 0 {
                continue;
            }
            //filter by direction
            if let Some(d) = direction_filter {
                if d != old_direction {
                    if old_level > 1 {
                        if let LightDirection::Directed(d) = d {
                            let invert = d.invert();
                            brighter.push((pos, invert));
                        }
                    }
                    continue;
                }
            }
            //remove light
            let light_type = world
                .game_data
                .blocks()
                .light_type(chunk_cache.chunk().data[pos].load());
            let reduced = if let LightType::Source(s) = *light_type {
                let reduced = s < old_level;
                if reduced {
                    light.set(s, LightDirection::SelfLit);
                    changed.insert(containing);
                }
                if s > 1 {
                    let direction = direction_filter.expect("unfiltered removal on source block");
                    if let LightDirection::Directed(d) = direction {
                        let invert = d.invert();
                        add.push(pos.facing(invert),
                                 (s - 1, LightDirection::Directed(invert)));
                    }
                }
                reduced
            } else {
                light.set(0, LightDirection::SelfLit);
                changed.insert(containing);
                true
            };
            //remove light from blocks lit by this block
            if old_level > 1 && reduced {
                for &d in &ALL_DIRECTIONS {
                    remove.push(pos.facing(d),
                                old_level - 1,
                                Some(LightDirection::Directed(d)));
                }
            }
        }
        // re-add bright blocks encountered
        for (pos, direction) in brighter {
            chunk_cache
                .load(pos.containing_chunk(), world)
                .expect("chunk unloaded during light update");
            let level = chunk_cache.chunk().artificial_light[pos].level();
            if level > 1 {
                add.push(pos.facing(direction),
                         (level - 1, LightDirection::Directed(direction)));
            }
        }
        remove.reset();
        mem::drop(remove);

        //refill from adjacent blocks
        for &pos in &*refill {
            let max = ALL_DIRECTIONS
                .iter()
                .map(|&d| {
                    let from_pos = pos.facing(d);
                    if chunk_cache.load(from_pos.containing_chunk(), world).is_ok() {
                        let level = chunk_cache.chunk().artificial_light[from_pos].level();
                        (d.invert(), level)
                    } else {
                        (d.invert(), 0)
                    }
                })
                .max_by_key(|&(_, l)| l)
                .unwrap();
            if max.1 > 1 {
                add.push(pos, (max.1 - 1, LightDirection::Directed(max.0)))
            }
        }
        refill.clear();
        mem::drop(refill);

        //add brightness
        while let Some((pos, new_light)) = add.pop() {
            let containing = pos.containing_chunk();
            if chunk_cache.load(containing, world).is_err() {
                continue;
            }
            let light = &chunk_cache.chunk().artificial_light[pos];
            let old_level = light.level();
            if old_level >= new_light.0 {
                continue;
            }
            let block = chunk_cache.chunk().data[pos].load();
            let light_type = *world.game_data().blocks().light_type(block);
            if light_type.is_opaque() {
                continue;
            }
            light.set(new_light.0, new_light.1);
            changed.insert(containing);
            if new_light.0 > 1 {
                for &d in &ALL_DIRECTIONS {
                    let adjacent = pos.facing(d);
                    add.push(adjacent, (new_light.0 - 1, LightDirection::Directed(d)));
                }
            }
        }
        add.reset();
        mem::drop(add);
        mem::drop(chunk_cache);

        //set render updates
        for c in changed {
            world.update_render(c);
        }
    }
}
