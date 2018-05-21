use std::sync::{Arc, Weak, atomic::Ordering};
use std::sync::mpsc::*;
use std::collections::hash_map::{HashMap, Entry};
use std::collections::VecDeque;
use world::{ChunkPos, Chunk};
use geometry::Direction;
use debug;

pub fn chunk_update_channel() -> (ChunkUpdateSender, ChunkUpdateReceiver) {
    let (send, rec) = channel();
    (
        sender::new_sender(send),
        ChunkUpdateReceiver {
            rec_buffer: VecDeque::new(),
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
    rec_buffer: VecDeque<ChunkPos>,
    chunks: HashMap<ChunkPos, ChunkRegion>,
}

impl ChunkUpdateReceiver {
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&ChunkRegion> {
        self.chunks.get(&pos)
    }
    pub fn poll_chunk_update(&mut self) -> Option<ChunkPos> {
        debug::manager().triggers.dump_graphics_chunk_cache.run_dump(|| { self.dump() });
        while let Some(c) = self.try_recv() {
            self.rec_buffer.push_back(c.pos);
            self.insert_chunk_to_map(c);
        }
        if let Some(p) = self.rec_buffer.pop_front() {
            //relaxed ordering because synchronised by channel
            self.chunks.get(&p).unwrap().center.is_in_update_queue.store(false, Ordering::Relaxed);
            Some(p)
        } else {
            None
        }
    }

    fn dump(&self) -> String {
        let mut lines = Vec::new();
        for (pos, reg) in &self.chunks {
            let mut line = String::new();
            for i in 0..6 {
                let facing = pos.facing(Direction::from_usize(i));
                let neighbour_state = match (reg.neighbours[i].upgrade(), self.chunks.get(&facing)) {
                    (None, None) => "None",
                    (Some(_), None) => "INVALID",
                    (None, Some(_)) => "MISSING",
                    (Some(n), Some(p)) => {
                        if Arc::ptr_eq(&n, &p.center) {
                            "Some"
                        } else {
                            "MISMATCH"
                        }
                    }
                };
                line += &format!("{:?}:{}, ", Direction::from_usize(i), neighbour_state);
            }
            lines.push((pos, line));
        }
        lines.sort_by_key(|l| l.0);
        let mut ret = String::new();
        for line in lines.iter().map(|&(ref p, ref l)| format!("{:4?}|{}\n", **p, l)) {
            ret = ret + &line;
        }
        ret
    }
    fn insert_chunk_to_map(&mut self, update: ChunkUpdate) {
        let update_surrounding = match self.chunks.entry(update.pos) {
            Entry::Vacant(e) => {
                let ret = Arc::downgrade(&update.chunk);
                e.insert(ChunkRegion { center: update.chunk, neighbours: Default::default() });
                Some(ret)
            }
            Entry::Occupied(mut e) => {
                if Arc::ptr_eq(&e.get().center, &update.chunk) {
                    None
                } else {
                    let ret = Arc::downgrade(&update.chunk);
                    e.insert(ChunkRegion { center: update.chunk, neighbours: Default::default() });
                    Some(ret)
                }
            }
        };
        if let Some(center) = update_surrounding {
            assert!(Weak::upgrade(&center).is_some());
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
                let facing = update.pos.facing(direction);
                if let Some(mut c) = self.chunks.get_mut(&facing) {
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
