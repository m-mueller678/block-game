use time::precise_time_ns;
use std::time::Duration;
use std::thread;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub const TICK_TIME: f64 = 1. / 20.;
pub const NANO_TICK_TIME: u64 = 1_000_000_000 / 20;
const AVERAGE_TICK_COUNT: u64 = 8;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Clone, Copy, Default)]
pub struct TickId(u64);

impl TickId {
    pub fn zero() -> Self {
        TickId(0)
    }
    pub fn next(self) -> Self {
        TickId(self.0 + 1)
    }
    pub fn ticks_since(self, other: Self) -> u64 {
        self.0 - other.0
    }
}

pub struct Timekeeper {
    tick: AtomicU64,
    previous_tick: AtomicU64,
    average_tick_nanoseconds: AtomicU64,
    next_tick_lock: Mutex<()>,
}

impl Timekeeper {
    pub fn new() -> Self {
        Timekeeper {
            tick: AtomicU64::new(0),
            previous_tick: AtomicU64::new(precise_time_ns()),
            average_tick_nanoseconds: AtomicU64::new(NANO_TICK_TIME),
            next_tick_lock: Mutex::new(()),
        }
    }

    pub fn current_tick(&self) -> TickId {
        TickId(self.tick.load(Ordering::Relaxed))
    }

    pub fn sub_tick_time(&self) -> f32 {
        let now = precise_time_ns();
        let duration = now - self.previous_tick.load(Ordering::Relaxed);
        (duration as f32 / self.average_tick_nanoseconds.load(Ordering::Relaxed) as f32).min(1.)
    }

    /// makes the TimeKeeper proceed to the next tick.
    /// if necessary waits until 50ms have elapsed since
    /// the last tick before switching to the next one.
    pub fn next_tick(&self) {
        let _lock = self.next_tick_lock.lock().unwrap();

        let mut now = precise_time_ns();
        let mut duration = now - self.previous_tick.load(Ordering::Relaxed);
        if NANO_TICK_TIME > duration {
            let wait = NANO_TICK_TIME - duration;
            thread::sleep(Duration::from_nanos(wait));
            duration = NANO_TICK_TIME;
            now = now + wait;
        }
        let mut avg = self.average_tick_nanoseconds.load(Ordering::Relaxed);
        avg *= AVERAGE_TICK_COUNT - 1;
        avg += duration;
        avg /= AVERAGE_TICK_COUNT;

        self.average_tick_nanoseconds.store(avg, Ordering::Relaxed);
        self.tick.fetch_add(1, Ordering::Relaxed);
        //previous tick being wrong between these two stores should not matter in practice
        self.previous_tick.store(now, Ordering::Relaxed);
    }
}
