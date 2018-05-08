use std::sync::{Arc, Weak, atomic::Ordering};
use std::sync::mpsc::*;
use std::collections::hash_map::{HashMap, Entry};
use world::{ChunkPos, Chunk};
use geometry::Direction;

pub fn chunk_update_channel() -> (ChunkUpdateSender, ChunkUpdateReceiver) {
    let (send, rec) = channel();
    (
        sender::new_sender(send),
        ChunkUpdateReceiver {
            rec,
            chunks: HashMap::new(),
        }
    )
}

pub struct ChunkUpdate {
    pub pos: ChunkPos,
    pub chunk: Arc<Chunk>,
}

#[derive(Clone)]
pub struct ChunkRegion {
    pub center: Arc<Chunk>,
    pub neighbours: [Weak<Chunk>; 6],
}

pub struct ChunkUpdateReceiver {
    rec: Receiver<ChunkUpdate>,
    chunks: HashMap<ChunkPos, ChunkRegion>,
}

impl ChunkUpdateReceiver {
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&ChunkRegion> {
        self.chunks.get(&pos)
    }
    pub fn poll_chunk_update(&mut self) -> Option<ChunkPos> {
        if let Some(c) = self.try_recv() {
            let pos = c.pos;
            //relaxed ordering because synchronised by channel
            c.chunk.is_in_update_queue.store(false, Ordering::Relaxed);
            self.insert_chunk_to_map(c);
            Some(pos)
        } else {
            None
        }
    }

    fn insert_chunk_to_map(&mut self, update: ChunkUpdate) {
        let update_surrounding = match self.chunks.entry(update.pos) {
            Entry::Vacant(e) => {
                Some(Arc::downgrade(
                    &e.insert(ChunkRegion { center: update.chunk, neighbours: Default::default() }).center
                ))
            }
            Entry::Occupied(mut e) => {
                if Arc::ptr_eq(&e.get().center, &update.chunk) {
                    None
                } else {
                    Some(Arc::downgrade(
                        &e.insert(ChunkRegion { center: update.chunk, neighbours: Default::default() }).center
                    ))
                }
            }
        };
        if let Some(center) = update_surrounding {
            let empty_weak: Weak<Chunk> = Weak::new();
            let mut neighbours = [
                empty_weak.clone(),
                empty_weak.clone(),
                empty_weak.clone(),
                empty_weak.clone(),
                empty_weak.clone(),
                empty_weak
            ];
            for i in 0..6 {
                let direction = Direction::from_usize(i);
                let pos = update.pos.facing(direction);
                if let Some(mut c) = self.chunks.get_mut(&pos) {
                    neighbours[i] = Arc::downgrade(&c.center);
                    c.neighbours[direction.invert() as usize] = center.clone();
                }
            }
            self.chunks.get_mut(&update.pos).unwrap().neighbours = neighbours;
        }
    }
    fn try_recv(&mut self) -> Option<ChunkUpdate> {
        match self.rec.try_recv() {
            Ok(update) => Some(update),
            Err(TryRecvError::Disconnected) => panic!("chunk graphics update channel disconnected"),
            Err(TryRecvError::Empty) => None,
        }
    }
}

pub use self::sender::ChunkUpdateSender;

mod sender {
    use super::*;

    pub struct ChunkUpdateSender {
        send: Sender<ChunkUpdate>
    }

    pub fn new_sender(send: Sender<ChunkUpdate>) -> ChunkUpdateSender {
        ChunkUpdateSender { send }
    }

    impl ChunkUpdateSender {
        // take mutable reference to guarantee Sync safety
        pub fn send(&mut self, pos: ChunkPos, chunk: &Arc<Chunk>) {
            use std::sync::atomic::Ordering;
            //relaxed ordering because synchronised by channel
            if !chunk.is_in_update_queue.swap(true, Ordering::Relaxed) {
                self.send.send(ChunkUpdate { pos, chunk: chunk.clone() }).unwrap();
            }
        }
    }

    // ChunkUpdateSender only supports operations on mutable references
    unsafe impl Sync for ChunkUpdateSender {}
}
