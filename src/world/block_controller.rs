use chashmap::CHashMap;
use std::collections::HashSet;
use super::{ChunkPos, BlockPos};
use std::sync::{Weak, Arc};
use std::thread::yield_now;

pub trait BlockController where Self: Send + Sync {
    fn on_insert(&self);
    fn on_remove(&self);
    fn on_unload(&self);
}

pub struct BlockControllerMap {
    controllers: CHashMap<BlockPos, Arc<BlockController>>,
    chunks: CHashMap<ChunkPos, HashSet<BlockPos>>,
}

pub enum CreateError {
    Occupied,
    NotLoaded,
}

impl BlockControllerMap {
    pub fn new() -> Self {
        BlockControllerMap {
            controllers: CHashMap::new(),
            chunks: CHashMap::new(),
        }
    }

    pub fn get_block_controller(&self, pos: BlockPos) -> Option<Weak<BlockController>> {
        self.controllers.get(&pos).map(|guard| Arc::downgrade(&*guard))
    }

    pub fn kill_block_controller(&self, pos: BlockPos) -> Option<Arc<BlockController>> {
        let chunk = pos.pos_in_chunk().0;
        if self.chunks.get_mut(&chunk)?.remove(&pos) {
            let controller = self.controllers.remove(&pos).expect("expected controller because listed in chunk");
            controller.on_remove();
            Some(controller)
        } else {
            None
        }
    }

    pub fn create_block_controller(&self, pos: BlockPos, controller: Arc<BlockController>) -> Result<(), CreateError> {
        if let Some(mut positions) = self.chunks.get_mut(&pos.pos_in_chunk().0) {
            if !positions.insert(pos) {
                return Err(CreateError::Occupied);
            }
            let mut insert = Some((controller, positions));
            loop {
                self.controllers.upsert(pos, || {
                    let insert = insert.take().unwrap();
                    drop(insert.1);
                    insert.0.on_insert();
                    insert.0
                }, |_| {});
                if insert.is_none() {
                    break;
                }
                yield_now();//wait for removal to be completed
            }
            Ok(())
        } else {
            Err(CreateError::NotLoaded)
        }
    }

    pub fn unload_chunk(&self, chunk: ChunkPos) {
        let positions = self.chunks.remove(&chunk).expect("chunk is loaded");
        for p in positions {
            self.controllers.remove(&p).unwrap().on_remove();
        }
    }

    pub fn enable_chunk(&self, pos: ChunkPos) {
        assert!(self.chunks.insert(pos, HashSet::new()).is_none())
    }
}