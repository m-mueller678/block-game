use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use block::*;
use geometry::{Direction, ALL_DIRECTIONS};
use world::World;
use super::*;


struct QueuedChunk {
    light_sources: Vec<(BlockPos, u8)>,
    pos: ChunkPos,
    data: Box<ChunkArray<AtomicBlockId>>,
}

pub struct Inserter {
    shared: Arc<(GameData, Mutex<Shared>)>,
    threads: Mutex<ThreadPool>,
}

struct Shared {
    pending: HashSet<ChunkPos>,
}

impl Inserter {
    pub fn new(gen: GameData) -> Self {
        Inserter {
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
            if lock.pending.contains(&pos){
                return;
            }
            lock.pending.insert(pos);
        }
        {
            let shared = Arc::clone(&self.shared);
            let pos = pos;
            self.threads.lock().unwrap().execute(move || {
                Self::generate_chunk(&*shared, pos, &*world.read())
            });
        }
    }

    pub fn cancel_insertion(&self, pos: ChunkPos) -> Result<(), ()> {
        let mut lock = self.shared.1.lock().unwrap();
        if lock.pending.remove(&pos) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn generate_chunk(shared: &(GameData, Mutex<Shared>), pos: ChunkPos, chunks: &ChunkMap) {
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
        let chunk = QueuedChunk {
            light_sources: sources,
            pos: pos,
            data: data,
        };

        {
            //make sure insertion has not been canceled
            let mut lock = shared.1.lock().unwrap();
            if !lock.pending.remove(&pos){
                return;
            }
        }

        let mut sources_to_trigger = UpdateQueue::new();
        let insert_pos = {
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
                ChunkMap::set_chunk_update(&chunks.chunk_updates, &*chunk, facing);
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
        chunks.update_render(insert_pos);
    }
}
