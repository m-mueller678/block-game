use chashmap::CHashMap;
use std::collections::HashSet;
use super::{ChunkPos, BlockPos};
use std::sync::{Weak, Arc};
use std::thread::yield_now;
use logging::root_logger;

pub trait BlockController where Self: Send + Sync {
    fn on_insert(&self);
    fn on_remove(&self);
    fn on_unload(&self);
    fn on_load(&self);
}

pub struct BlockControllerMap {
    controllers: CHashMap<BlockPos, Arc<BlockController>>,
    chunks: CHashMap<ChunkPos, HashSet<BlockPos>>,
}

#[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn get_block_controller(&self, pos: BlockPos) -> Option<Weak<BlockController>> {
        self.controllers.get(&pos).map(|guard| Arc::downgrade(&*guard))
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn create_block_controller(&self, pos: BlockPos, controller: Arc<BlockController>) -> Result<(), CreateError> {
        //lock chunk entry
        if let Some(mut positions) = self.chunks.get_mut(&pos.pos_in_chunk().0) {
            if !positions.insert(pos) {
                return Err(CreateError::Occupied);
            }
            Self::insert_controller(&self.controllers, controller, pos, move |c| {
                drop(positions);
                c.on_insert();
            });
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

    pub fn load_chunk<I: Iterator<Item=(BlockPos, Arc<BlockController>)>>(&self, chunk: ChunkPos, controllers: I) {
        let mut positions = HashSet::new();
        //lock chunk entry
        self.chunks.upsert(chunk, || {
            for bc in controllers {
                if !positions.insert(bc.0) {
                    error!(root_logger(), "block controller occupied on chunk load {:?}", bc.0);
                    continue;
                }
                Self::insert_controller(&self.controllers, bc.1, bc.0, |c| c.on_load());
            }
            positions
        }, |_| panic!("{:?} already loaded", chunk));
    }

    /// crate controller entry
    /// release is called immediately after acquiring the controller entry lock
    /// chunk entry should stay locked until release is called or this function returns
    /// does not call BlockController::on_insert
    fn insert_controller<R: FnOnce(&Arc<BlockController>)>(
        map: &CHashMap<BlockPos, Arc<BlockController>>,
        controller: Arc<BlockController>,
        pos: BlockPos,
        release: R,
    ) {
        let mut insert = Some((controller, release));
        loop {
            //try creating controller entry
            map.upsert(pos, || {
                let insert = insert.take().unwrap();
                insert.1(&insert.0);
                insert.0
            }, |_| {});
            if insert.is_none() {
                break;
            }
            yield_now();//wait for removal to be completed
        }
    }
}