use std::collections::hash_map::*;
use std::collections::hash_set::*;
use std::sync::{Arc, Mutex};
use super::{ChunkPos, World};

struct InnerMap {
    map: HashMap<ChunkPos, u32>,
    new_loaded: HashSet<ChunkPos>,
    new_unloaded: HashSet<ChunkPos>,
}

impl InnerMap {
    fn inc(&mut self, pos: ChunkPos) {
        match self.map.entry(pos) {
            Entry::Occupied(mut o) => (*o.get_mut()) += 1,
            Entry::Vacant(v) => {
                v.insert(1);
                if !self.new_unloaded.remove(&pos) {
                    self.new_loaded.insert(pos);
                }
            }
        }
    }
    fn dec(&mut self, pos: ChunkPos) {
        if let Entry::Occupied(mut o) = self.map.entry(pos) {
            let new_count = {
                let count: &mut _ = o.get_mut();
                (*count) -= 1;
                *count
            };
            if new_count == 0 {
                o.remove();
                if !self.new_loaded.remove(&pos) {
                    self.new_unloaded.insert(pos);
                }
            }
        } else {
            panic!("load map count decreased below zero");
        }
    }
    fn apply_to_world(&mut self, map: &World) {
        for pos in self.new_unloaded.drain() {
            if map.chunks.remove_chunk(pos).is_none() {
                map.inserter.cancel_insertion(pos).unwrap();
            } else {
                map.block_controllers.unload_chunk(pos);
            }
        }
        map.inserter.push_to_world(&map.chunks, |pos| {
            map.block_controllers.enable_chunk(pos);
        });
        for pos in self.new_loaded.drain() {
            map.inserter.insert(pos, &map.chunks);
        }
    }
}

pub struct LoadMap {
    loaded: Arc<Mutex<InnerMap>>,
}

impl LoadMap {
    pub fn new() -> Self {
        LoadMap {
            loaded: Arc::new(Mutex::new(InnerMap {
                map: HashMap::new(),
                new_loaded: HashSet::new(),
                new_unloaded: HashSet::new(),
            })),
        }
    }
    pub fn load_cube(&self, center: ChunkPos, radius: i32) -> LoadGuard {
        assert!(radius >= 0);
        let mut lock = self.loaded.lock().unwrap();
        for x in center[0] - radius..center[0] + radius + 1 {
            for y in center[1] - radius..center[1] + radius + 1 {
                for z in center[2] - radius..center[2] + radius + 1 {
                    lock.inc(ChunkPos([x, y, z]));
                }
            }
        }
        LoadGuard {
            map: Arc::clone(&self.loaded),
            center: center,
            size: radius,
        }
    }
    pub fn apply_to_world(&self, map: &World) {
        self.loaded.lock().unwrap().apply_to_world(map);
    }
}

pub struct LoadGuard {
    map: Arc<Mutex<InnerMap>>,
    center: ChunkPos,
    size: i32,
}

impl Drop for LoadGuard {
    fn drop(&mut self) {
        let mut lock = self.map.lock().unwrap();
        for x in self.center[0] - self.size..self.center[0] + self.size + 1 {
            for y in self.center[1] - self.size..self.center[1] + self.size + 1 {
                for z in self.center[2] - self.size..self.center[2] + self.size + 1 {
                    lock.dec(ChunkPos([x, y, z]));
                }
            }
        }
    }
}
