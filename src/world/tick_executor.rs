use std::sync::Mutex;
use world::World;
use world::timekeeper::TickId;

pub enum TickFunctionResult {
    Keep,
    Drop,
}

pub type TickFunction = Box<FnMut(&World, TickId) -> TickFunctionResult + Send>;

#[derive(Default)]
pub struct TickExecutor {
    functions: Mutex<Vec<TickFunction>>,
    add: Mutex<Vec<TickFunction>>,
}

impl TickExecutor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn run(&self, world: &World, now: TickId) {
        let mut add = self.add.lock().unwrap();
        let mut functions = self.functions.lock().unwrap();
        functions.append(&mut *add);
        drop(add);
        let mut i = 0;
        while i < functions.len() {
            use std::ops::IndexMut;
            match functions.index_mut(i)(world, now) {
                TickFunctionResult::Keep => { i += 1 }
                TickFunctionResult::Drop => { functions.swap_remove(i); }
            }
        }
    }

    /// if called from tick function will not execute in current tick
    pub fn add(&self, f: TickFunction) {
        self.add.lock().unwrap().push(f);
    }
}