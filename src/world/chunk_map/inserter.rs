use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use block::*;
use geometry::{Direction, ALL_DIRECTIONS};
use super::*;

struct QueuedChunk {
    light_sources: Vec<(BlockPos, u8)>,
    pos: ChunkPos,
    data: Box<ChunkArray<AtomicBlockId>>,
}

pub struct Inserter {
    shared: Arc<(GameData, Mutex<InsertBuffer>)>,
    threads: Mutex<ThreadPool>,
}

struct InsertBuffer {
    chunks: VecDeque<(QueuedChunk)>,
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

    pub fn insert(&self, pos: ChunkPos, world: &ChunkMap) {
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

    pub fn cancel_insertion(&self, pos: ChunkPos) -> Result<(), ()> {
        let mut lock = self.shared.1.lock().unwrap();
        if let Some(pending_index) = lock.pending.iter().position(|p| *p == pos) {
            lock.pending.swap_remove(pending_index);
            Ok(())
        } else if let Some(generated_index) = lock.chunks.iter().position(|qc| qc.pos == pos) {
            lock.chunks.swap_remove_back(generated_index);
            Ok(())
        } else {
            Err(())
        }
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

    pub fn push_to_world(&self, chunks: &mut ChunkMap) {
        let mut sources_to_trigger = UpdateQueue::new();
        let insert_pos = if let Some(chunk) = self.shared.1.lock().unwrap().chunks.pop_front() {
            chunks.chunks.insert(
                [chunk.pos[0], chunk.pos[1], chunk.pos[2]],
                Box::new(Chunk {
                    natural_light: Default::default(),
                    data: *chunk.data,
                    artificial_light: Default::default(),
                    update_render: AtomicBool::new(false),
                }),
            );
            for source in &chunk.light_sources {
                sources_to_trigger.push(source.1, source.0, None);
            }
            chunk.pos
        } else {
            return;
        };

        let cs = CHUNK_SIZE as i32;
        let mut sky_light = UpdateQueue::new();
        if !chunks.chunk_loaded(insert_pos.facing(Direction::PosY)) {
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
            if let Some(chunk) = chunks.borrow_chunk(facing) {
                chunks.trigger_chunk_face_brightness(
                    facing,
                    face.invert(),
                    &mut sources_to_trigger,
                    &mut sky_light,
                );
                chunk.update_render.store(true, Ordering::Release);
            }
        }
        increase_light(
            &mut chunks.artificial_lightmap(insert_pos),
            sources_to_trigger,
        );
        increase_light(&mut chunks.natural_lightmap(insert_pos), sky_light);

        //block natural light in chunk below
        if chunks.chunk_loaded(insert_pos.facing(Direction::NegY)) {
            let mut relight = RelightData::new();
            let mut lm = chunks.natural_lightmap(insert_pos);
            let inserted_cache = ChunkCache::new(insert_pos, chunks).unwrap();
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let abs_pos = BlockPos(
                        [
                            insert_pos[0] * cs + x as i32,
                            insert_pos[1] * cs - 1,
                            insert_pos[2] * cs + z as i32,
                        ],
                    );
                    if inserted_cache.chunk.natural_light[[x, 0, z]].level() != MAX_NATURAL_LIGHT {
                        remove_light_rec(&mut lm, abs_pos, Direction::NegY, &mut relight);
                    }
                }
            }
            increase_light(
                &mut chunks.natural_lightmap(insert_pos.facing(Direction::NegY)),
                relight.build_queue(&mut lm),
            );
        }
    }
}
