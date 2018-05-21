use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct DebugTrigger {
    pending: AtomicBool,
}

impl DebugTrigger {
    pub fn reset(&self) -> bool {
        self.pending.swap(false, Ordering::Relaxed)
    }

    pub fn run_dump<F: FnOnce() -> String>(&self, f: F) {
        if self.reset() {
            println!("{}", f());
        }
    }

    pub fn trigger(&self) {
        self.pending.store(true, Ordering::Relaxed);
    }
}

macro_rules! declare_triggers {
    ( $ ( $ name: ident), * ) => {
        #[derive(Default)]
        pub struct TriggerList {
            $(
                pub $name:DebugTrigger,
            )*
        }

        impl TriggerList{
            pub fn trigger(&self,trigger_name:&str)->Result<(),()>{
                match trigger_name{
                    $(
                        stringify!($name)=>{self.$name.trigger();Ok(())},
                    )*
                    _=>{Err(())},
                }
            }
        }
    }
}

declare_triggers!(
dump_graphics_chunk_cache
);
