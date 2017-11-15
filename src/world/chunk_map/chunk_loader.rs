use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::collections::HashSet;
use geometry::ALL_DIRECTIONS;
use block::LightType;
use world::{BlockPos, ChunkPos};
use world::chunk_map::chunk::*;
use block::AtomicBlockId;
use logging;
use super::ChunkMap;
use super::lighting::LightDirection;

pub enum ChunkLighting {
    New { sources: Vec<(BlockPos, u8)> },
}

pub struct QueuedChunk {
    pub pos: ChunkPos,
    pub lighting: ChunkLighting,
    pub blocks: ChunkArray<AtomicBlockId>,
}

pub struct ChunkLoader {
    load: Mutex<Vec<Box<QueuedChunk>>>,
    unload: Mutex<Vec<ChunkPos>>,
    logger: logging::Logger,
    enabled: Mutex<HashSet<ChunkPos>>,
}

impl ChunkLoader {
    pub fn new() -> Self {
        ChunkLoader {
            load: Mutex::new(Vec::new()),
            unload: Mutex::new(Vec::new()),
            logger: logging::root_logger().clone(),
            enabled: Mutex::new(HashSet::new()),
        }
    }

    /// returns true if the chunk was enabled before this call
    pub fn enable_chunk(&self, pos: ChunkPos) -> bool {
        !self.enabled.lock().unwrap().insert(pos)
    }

    /// returns true if the chunk was enabled before this call
    pub fn disable_chunk(&self, pos: ChunkPos) -> bool {
        let ret = self.enabled.lock().unwrap().remove(&pos);
        self.unload.lock().unwrap().push(pos);
        ret
    }

    /// queue this chunk for insertion if it is enabled
    pub fn chunk_ready(&self, chunk: Box<QueuedChunk>) {
        self.load.lock().unwrap().push(chunk);
    }

    pub fn apply(&self, world: &ChunkMap) {
        {
            //unload
            let mut lock = self.unload.lock().unwrap();
            for pos in &*lock {
                world.chunks.remove(pos);
            }
            lock.clear();
        }
        {
            //load
            //calculate light
            let light = world.light();
            let mut lock = self.load.lock().unwrap();
            {
                let enabled = self.enabled.lock().unwrap();
                lock.retain(|qc| enabled.contains(&qc.pos));
            }
            for chunk in &*lock {
                match chunk.lighting {
                    ChunkLighting::New { ref sources } => {
                        for s in sources {
                            light.block_light_changed((0, LightDirection::SelfLit),
                                                      &LightType::Source(s.1),
                                                      s.0);
                        }
                        for d in &ALL_DIRECTIONS {
                            let adjacent = chunk.pos.facing(*d);
                            if let Some(chunk) = world.chunks.get(&*adjacent) {
                                light.trigger_artificial_chunk_face(&chunk.artificial_light,
                                                                    adjacent,
                                                                    d.invert())
                            }
                        }
                    }
                }
            }
            //insert chunks
            let chunks = mem::replace(&mut *lock, Vec::new());
            for chunk in chunks {
                let artificial_light = match chunk.lighting {
                    ChunkLighting::New { .. } => Default::default(),
                };
                let QueuedChunk { pos, blocks, .. } = *chunk;
                let replaced = world
                    .chunks
                    .insert(*pos,
                            Box::new(Chunk {
                                         data: blocks,
                                         artificial_light,
                                         natural_light: Default::default(),
                                         update_render: AtomicBool::new(false),
                                     }));
                world.update_render(pos);
                if replaced.is_some() {
                    error!(&self.logger,
                           "inserting chunk at {:?}, which was already loaded",
                           pos);
                }
            }
        }
    }
}
