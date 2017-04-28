use std::mem::replace;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use block::*;
use geometry::{Direction, ALL_DIRECTIONS};
use super::*;
use super::generator::Generator;
use super::lighting::*;
use super::atomic_light::*;

struct QueuedChunk {
    light_sources: Vec<(BlockPos, u8)>,
    pos: ChunkPos,
    data: [AtomicBlockId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

pub struct Inserter {
    shared: Arc<(Generator,Mutex<InsertBuffer>)>,
    columns: Vec<(i32, i32, ChunkColumn)>,
    threads: ThreadPool,
}

struct InsertBuffer {
    chunks: VecDeque<(QueuedChunk)>,
    pending: Vec<ChunkPos>,
}

impl Inserter {
    pub fn new(gen: Generator) -> Self {
        Inserter {
            shared: Arc::new((gen,Mutex::new(InsertBuffer {
                chunks: VecDeque::new(),
                pending: Vec::new(),
            }))),
            columns: Vec::new(),
            threads: ThreadPool::new_with_name("chunk generator".into(), 3),
        }
    }

    pub fn insert(&mut self, pos: &ChunkPos, world: &WorldReadGuard) {
        if world.chunk_loaded(pos) {
            return;
        }
        {
            let mut lock = self.shared.1.lock().unwrap();
            if lock.chunks.iter().any(|chunk| chunk.pos == *pos) || lock.pending.iter().any(|p| p == pos) {
                return;
            }
            lock.pending.push(pos.clone());
        }
        {
            let shared = self.shared.clone();
            let pos = pos.clone();
            let blocks = world.blocks.clone();
            self.threads.execute(move || Self::generate_chunk(shared, pos, blocks));
        }
        if !world.columns.contains_key(&[pos[0], pos[2]]) {
            if !self.columns.iter().any(|&(x, z, _)| x == pos[0] && z == pos[2]) {
                self.columns.push((pos[0], pos[2], ChunkColumn::new()))
            }
        }
    }

    fn generate_chunk(
        shared: Arc<(Generator,Mutex<InsertBuffer>)>,
        pos: ChunkPos,
        blocks: Arc<BlockRegistry>)
    {
        let data = shared.0.gen_chunk(&pos);
        let sources = (0..(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE)).filter_map(|i| {
            match *blocks.light_type(data[i]) {
                LightType::Source(l) => Some((BlockPos([
                    pos[0] * CHUNK_SIZE as i32 + (i / CHUNK_SIZE / CHUNK_SIZE) as i32,
                    pos[1] * CHUNK_SIZE as i32 + (i / CHUNK_SIZE % CHUNK_SIZE) as i32,
                    pos[2] * CHUNK_SIZE as i32 + (i % CHUNK_SIZE) as i32
                ]), l)),
                LightType::Opaque | LightType::Transparent => None,
            }
        }).collect();
        let insert = QueuedChunk {
            light_sources: sources,
            pos: pos.clone(),
            data: AtomicBlockId::init_chunk(&data),
        };
        {
            let mut lock = shared.1.lock().unwrap();
            lock.chunks.push_back(insert);
            let index = lock.pending.iter().position(|p| *p == pos).unwrap();
            lock.pending.swap_remove(index);
        }
    }

    pub fn push_to_world(&mut self, chunks: &mut WorldWriteGuard) {
        self.flush_columns(chunks);

        let mut sources_to_trigger = UpdateQueue::new();
        let insert_pos = if let Some(chunk) = self.shared.1.lock().unwrap().chunks.pop_front() {
            let column = chunks.columns.get_mut(&[chunk.pos[0], chunk.pos[2]]).unwrap();
            column.insert(chunk.pos[1], Chunk {
                natural_light: LightState::init_dark_chunk(),
                data: chunk.data,
                artificial_light: LightState::init_dark_chunk(),
                update_render: AtomicBool::new(false)
            });
            for source in chunk.light_sources.iter() {
                sources_to_trigger.push(source.1, source.0.clone(), None);
            }
            chunk.pos.clone()
        } else {
            return;
        };

        let cs = CHUNK_SIZE as i32;
        let mut sky_light = UpdateQueue::new();
        if !chunks.chunk_loaded(&insert_pos.facing(Direction::PosY)) {
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let abs_pos = BlockPos([
                        cs * insert_pos[0] + x as i32,
                        cs * insert_pos[1] + cs - 1,
                        cs * insert_pos[2] + z as i32,
                    ]);
                    sky_light.push(MAX_NATURAL_LIGHT, abs_pos, Some(Direction::NegY));
                }
            }
        }
        for face in ALL_DIRECTIONS.iter() {
            let facing = insert_pos.facing(*face);
            if let Some(chunk) = chunks.borrow_chunk(&facing) {
                chunks.trigger_chunk_face_brightness(&facing, face.invert(), &mut sources_to_trigger, &mut sky_light);
                chunk.update_render.store(true, Ordering::Release);
            }
        }
        increase_light(&mut chunks.artificial_lightmap(insert_pos.clone()), sources_to_trigger);
        increase_light(&mut chunks.natural_lightmap(insert_pos.clone()), sky_light);

        //block natural light in chunk below
        if chunks.chunk_loaded(&insert_pos.facing(Direction::NegY)) {
            let mut relight = RelightData::new();
            let mut lm = chunks.natural_lightmap(insert_pos.clone());
            let inserted_cache = ChunkCache::new(insert_pos.clone(), &chunks).unwrap();
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let abs_pos = BlockPos([
                        insert_pos[0] * cs + x as i32,
                        insert_pos[1] * cs - 1,
                        insert_pos[2] * cs + z as i32,
                    ]);
                    if inserted_cache.chunk.natural_light[chunk_index(&[x, 0, z])].level() != MAX_NATURAL_LIGHT {
                        remove_light_rec(&mut lm, abs_pos, Direction::NegY, &mut relight);
                    }
                }
            }
            increase_light(&mut chunks.natural_lightmap(insert_pos.facing(Direction::NegY)), relight.build_queue(&mut lm));
        }
    }

    fn flush_columns(&mut self, chunks: &mut ChunkMap) {
        let columns = replace(&mut self.columns, Vec::new());
        for (x, z, col) in columns.into_iter() {
            let new = chunks.columns.insert([x, z], col).is_none();
            assert!(new);
        }
    }
}
