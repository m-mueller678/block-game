use geometry::{Direction, ALL_DIRECTIONS};
use world::BlockPos;

pub type Light = (u8, Option<Direction>);

pub struct UpdateQueue {
    levels: Vec<Vec<(BlockPos, Option<Direction>)>>
}

impl UpdateQueue {
    pub fn single(level: u8, pos: BlockPos, direction: Option<Direction>) -> Self {
        let mut uq = Self::new();
        uq.push(level, pos, direction);
        uq
    }
    pub fn new() -> Self {
        UpdateQueue {
            levels: Vec::new()
        }
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
    fn is_opaque(&mut self, pos: &BlockPos) -> bool;
    fn get_light(&mut self, pos: &BlockPos) -> Light;
    fn set_light(&mut self, pos: &BlockPos, Light);
    // level > 1
    fn compute_light_to(&mut self, direction: Direction, level: u8) -> u8;
    fn internal_light(&mut self, block: &BlockPos) -> u8;
}

pub fn remove_light_rec<LM: LightMap>(lm: &mut LM, pos: BlockPos, light_direction: Direction, brighter: &mut UpdateQueue) {
    let light = lm.get_light(&pos);
    if light.1 == Some(light_direction) {
        let internal_light = lm.internal_light(&pos);
        if internal_light < light.0 {
            if internal_light > 1 {
                lm.set_light(&pos, (0, None));
                brighter.push(internal_light, pos.clone(), None);
            } else {
                lm.set_light(&pos, (internal_light, None));
            }
            for d in ALL_DIRECTIONS.iter() {
                remove_light_rec(lm, pos.facing(*d), *d, brighter);
            }
        }
    } else if light.0 > 1 {
        let inverse_direction = light_direction.invert();
        let block_from = pos.facing(inverse_direction);
        brighter.push(lm.compute_light_to(inverse_direction, light.0), block_from, Some(inverse_direction))
    }
}

pub fn relight<LM: LightMap>(lm: &mut LM, pos: &BlockPos) {
    let mut updates = UpdateQueue::new();
    let internal_light = lm.internal_light(pos);
    if internal_light > 0 {
        updates.push(internal_light, pos.clone(), None);
    }
    for d in ALL_DIRECTIONS.iter() {
        let adjacent_light = lm.get_light(&pos.facing(d.invert())).0;
        if adjacent_light > 1 {
            updates.push(lm.compute_light_to(*d, adjacent_light), pos.clone(), Some(*d));
        }
    }
    increase_light(lm, updates);
}

pub fn remove_and_relight<LM: LightMap>(lm: &mut LM, positions: &[BlockPos]) {
    let mut update_brightness = UpdateQueue::new();
    for pos in positions.iter() {
        let internal_light = lm.internal_light(pos);
        if internal_light > 1 {
            lm.set_light(pos, (0, None));
            update_brightness.push(internal_light, pos.clone(), None);
        } else {
            lm.set_light(pos, (internal_light, None));
        }
        for d in ALL_DIRECTIONS.iter() {
            remove_light_rec(lm, pos.facing(*d), *d, &mut update_brightness);
        }
    }
    increase_light(lm, update_brightness);
}

pub fn increase_light<LM: LightMap>(lm: &mut LM, mut to_update: UpdateQueue) {
    while !to_update.levels.is_empty() {
        let update_len = to_update.levels.len();
        let level = update_len as u8;
        if level == 0 {
            continue;
        }
        while let Some(current_pos) = to_update.levels[update_len - 1].pop() {
            if lm.is_opaque(&current_pos.0) {
                continue;
            }
            let light = lm.get_light(&current_pos.0);
            if level > light.0 {
                lm.set_light(&current_pos.0, (level, current_pos.1));
                if level > 1 {
                    for d in ALL_DIRECTIONS.iter() {
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
