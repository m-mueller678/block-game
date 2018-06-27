use std::collections::VecDeque;
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use threadpool::ThreadPool;
use block::*;
use super::*;
use world::World;

pub struct QueuedChunk {
    light_sources: Vec<(BlockPos, u8)>,
    pos: ChunkPos,
    data: Box<ChunkArray<AtomicBlockId>>,
    block_controllers: Vec<(BlockPos, Arc<BlockController>)>,
}

pub struct Inserter {
    shared: Arc<(GameData, Mutex<InsertBuffer>)>,
    threads: Mutex<ThreadPool>,
}

struct InsertBuffer {
    chunks: VecDeque<QueuedChunk>,
    pending: Vec<ChunkPos>,
}

impl Inserter {
    pub fn new(gen: GameData) -> Self {
        Inserter {
            shared: Arc::new((
                gen,
                Mutex::new(InsertBuffer {
                    chunks: VecDeque::new(),
                    pending: Vec::new(),
                }),
            )),
            threads: Mutex::new(ThreadPool::with_name("chunk generator".into(), 3)),
        }
    }

    /// request a chunk for insertion
    /// this chunk will eventually become ready for poll
    pub fn request(&self, pos: ChunkPos, world: &ChunkMap) {
        if world.chunk_loaded(pos) {
            return;
        }
        {
            let mut lock = self.shared.1.lock().unwrap();
            if lock.chunks.iter().any(|chunk| chunk.pos == pos) ||
                lock.pending.iter().any(|p| *p == pos)
                {
                    return;
                }
            lock.pending.push(pos);
        }
        {
            let shared = Arc::clone(&self.shared);
            let pos = pos;
            self.threads.lock().unwrap().execute(move || {
                Self::generate_chunk(shared, pos)
            });
        }
    }

    /// cancel a previously requested chunk
    /// chunks which have not been requested will be ignored
    pub fn cancel(&self, pos: ChunkPos) {
        let mut lock = self.shared.1.lock().unwrap();
        if let Some(pending_index) = lock.pending.iter().position(|p| *p == pos) {
            lock.pending.swap_remove(pending_index);
        }
    }

    /// poll chunks ready for insertion
    /// the returned operation will be performed on each chunk
    /// chunks may become ready without being requested
    /// assumes no other load or unload operations are performed while this function is running
    pub fn poll<F: FnMut(ChunkPos) -> ChunkOperation, >(&self, world: &World, mut chunk_decider: F) {
        let mut lock = self.shared.1.lock().unwrap();
        while let Some(chunk) = lock.chunks.pop_front() {
            match chunk_decider(chunk.pos) {
                ChunkOperation::Discard => {}
                ChunkOperation::Insert => {
                    Self::insert_chunk(world, chunk);
                    break;
                }
            }
        }
    }

    fn insert_chunk(world: &World, queued_chunk: QueuedChunk) {
        let chunk = Arc::new(Chunk {
            natural_light: Default::default(),
            data: *queued_chunk.data,
            artificial_light: Default::default(),
            is_in_update_queue: AtomicBool::new(false),
        });
        world.chunks.insert_chunk(queued_chunk.pos, chunk, &queued_chunk.light_sources);
        world.block_controllers.load_chunk(queued_chunk.pos, queued_chunk.block_controllers.into_iter());
    }

    fn generate_chunk(shared: Arc<(GameData, Mutex<InsertBuffer>)>, pos: ChunkPos) {
        let data = shared.0.generator().gen_chunk(pos);
        let mut sources = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    match *shared.0.blocks().light_type(data[[x, y, z]].load()) {
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
        let insert = QueuedChunk {
            block_controllers: Vec::new(),
            light_sources: sources,
            pos: pos,
            data: data,
        };
        {
            let mut lock = shared.1.lock().unwrap();
            if let Some(index) = lock.pending.iter().position(|p| *p == pos) {
                lock.pending.swap_remove(index);
                lock.chunks.push_back(insert);
            }
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum ChunkOperation {
    Discard,
    Insert,
}