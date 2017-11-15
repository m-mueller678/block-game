use std::collections::hash_map::*;
use std::sync::{Arc, Mutex, Weak};
use logging;
use super::{ChunkPos, World};

struct InnerMap {
    map: HashMap<ChunkPos, u32>,
    world: Weak<World>,
}

impl InnerMap {
    fn inc(&mut self, pos: ChunkPos) {
        if let Some(world) = self.world.upgrade() {
            match self.map.entry(pos) {
                Entry::Occupied(mut o) => (*o.get_mut()) += 1,
                Entry::Vacant(v) => {
                    v.insert(1);
                    let world2 = Arc::clone(&world);
                    if !world2.read().chunk_loader().enable_chunk(pos) {
                        world2.inserter.insert(pos, world);
                    } else {
                        error!(logging::root_logger(),
                               "chunk {:?} was already enabled",
                               pos);
                    }
                }
            }
        }
    }
    fn dec(&mut self, pos: ChunkPos) {
        if let Some(world) = self.world.upgrade() {
            if let Entry::Occupied(mut o) = self.map.entry(pos) {
                let new_count = {
                    let count: &mut _ = o.get_mut();
                    (*count) -= 1;
                    *count
                };
                if new_count == 0 {
                    o.remove();
                    if !world.read().chunk_loader().disable_chunk(pos) {
                        error!(logging::root_logger(), "chunk {:?} was not enabled", pos);
                    }
                }
            } else {
                panic!("load map count decreased below zero");
            }
        };
    }
}

pub struct LoadMap {
    loaded: Arc<Mutex<InnerMap>>,
}

impl LoadMap {
    pub fn new(world: Weak<World>) -> Self {
        LoadMap {
            loaded: Arc::new(Mutex::new(InnerMap {
                                            world,
                                            map: HashMap::new(),
                                        })),
        }
    }

    pub fn reset_world(&self, world: Weak<World>) {
        let mut lock = self.loaded.lock().unwrap();
        assert!(lock.map.is_empty());
        lock.world = world;
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
