use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use block::*;
use world::World;
use super::*;

pub struct PreInserter {
    shared: Arc<(GameData, Mutex<Shared>)>,
    threads: Mutex<ThreadPool>,
}

struct Shared {
    pending: HashSet<ChunkPos>,
}

fn find_light_sources(blocks: &BlockRegistry, chunk: &ChunkArray<AtomicBlockId>, pos: ChunkPos) -> Vec<(BlockPos, u8)> {
    let mut sources = Vec::new();
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                match *blocks.light_type(chunk[[x, y, z]].load()) {
                    LightType::Source(l) => {
                        sources.push((
                            BlockPos(
                                [
                                    pos[0] * CHUNK_SIZE as i32 + x as i32,
                                    pos[1] * CHUNK_SIZE as i32 + y as i32,
                                    pos[2] * CHUNK_SIZE as i32 + z as i32,
                                ],
                            ),
                            l,
                        ))
                    }
                    LightType::Opaque | LightType::Transparent => {}
                }
            }
        }
    }
    sources
}

impl PreInserter {
    pub fn new(gen: GameData) -> Self {
        PreInserter {
            shared: Arc::new((
                gen,
                Mutex::new(Shared {
                    pending: HashSet::new(),
                }),
            )),
            threads: Mutex::new(ThreadPool::with_name("chunk generator".into(), 3)),
        }
    }

    pub fn insert(&self, pos: ChunkPos, world: Arc<World>) {
        if world.read().chunk_loaded(pos) {
            return;
        }
        {
            let mut lock = self.shared.1.lock().unwrap();
            if lock.pending.contains(&pos) {
                return;
            }
            lock.pending.insert(pos);
        }
        {
            let shared = Arc::clone(&self.shared);
            let pos = pos;
            self.threads.lock().unwrap().execute(move || {
                Self::generate_chunk(&*shared, pos, world.read().chunk_loader())
            });
        }
    }

    fn generate_chunk(shared: &(GameData, Mutex<Shared>), pos: ChunkPos, chunks: &chunk_loader::ChunkLoader) {
        let data = shared.0.generator().gen_chunk(pos);
        let sources = find_light_sources(shared.0.blocks(), &data, pos);
        chunks.chunk_ready(Box::new(chunk_loader::QueuedChunk {
            pos,
            lighting: chunk_loader::ChunkLighting::New{ sources },
            blocks: *data,
        }));
    }
}
