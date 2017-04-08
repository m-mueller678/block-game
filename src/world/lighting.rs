use geometry::{Direction, ALL_DIRECTIONS};

pub type Light = (u8, Direction);

pub trait LightMap {
    fn is_opaque(&mut self, pos: &[i32; 3]) -> bool;
    fn get_light(&mut self, pos: &[i32; 3]) -> Light;
    fn set_light(&mut self, pos: &[i32; 3], Light);
}

pub fn decrease_light<LM: LightMap>(lm: &mut LM, pos: &[i32; 3], new_light: Light) {
    let old_light = lm.get_light(pos);
    remove_light(lm, vec![(*pos, old_light.1)], old_light.0);
    increase_light(lm, vec![(*pos, new_light.1)], new_light.0);
}

pub fn remove_light<LM: LightMap>(lm: &mut LM, mut positions: Vec<([i32; 3], Direction)>, mut level: u8) {
    let mut brighter = Vec::new();
    loop {
        let mut remove_next = Vec::new();
        for pos in positions.iter() {
            let light = lm.get_light(&pos.0);
            if light == (level, pos.1) {
                lm.set_light(&pos.0, (0, Direction::PosX));
                if level > 1 {
                    for d in ALL_DIRECTIONS.iter() {
                        remove_next.push((d.apply_to_pos(pos.0), *d))
                    }
                }
            } else if light.0 > level + 2 {
                brighter.push(pos.0);
            }
        }
        if remove_next.is_empty() {
            break;
        } else {
            level -= 1;
            positions = remove_next;
        }
    }
    for p in brighter.iter() {
        let light = lm.get_light(p);
        let mut adjacent = Vec::new();
        for d in ALL_DIRECTIONS.iter() {
            adjacent.push((d.apply_to_pos(*p), *d));
        }
        increase_light(lm, adjacent, light.0 - 1);
    }
}

pub fn increase_light<LM: LightMap>(lm: &mut LM, mut positions: Vec<([i32; 3], Direction)>, mut level: u8) {
    loop {
        let mut increase_next = Vec::new();
        for pos in positions {
            if lm.is_opaque(&pos.0) {
                continue;
            }
            let light = lm.get_light(&pos.0);
            if level > light.0 {
                lm.set_light(&pos.0, (level, pos.1));
                if level > 1 {
                    for d in ALL_DIRECTIONS.iter() {
                        increase_next.push((d.apply_to_pos(pos.0), *d));
                    }
                }
            }
        }
        if increase_next.is_empty() {
            break;
        }
        positions = increase_next;
        level -= 1;
    }
}
