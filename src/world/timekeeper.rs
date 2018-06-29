use time::{SteadyTime, Duration};
use std::thread;
use TICK_TIME;

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
    tick: TickId,
    previous_tick: SteadyTime,
    average_tick_nanoseconds: u64,
}

impl Timekeeper {
    pub fn new() -> Self {
        Timekeeper {
            tick: TickId::zero(),
            previous_tick: SteadyTime::now(),
            average_tick_nanoseconds: (TICK_TIME * 1e9) as u64,
        }
    }

    pub fn current_tick(&self) -> TickId {
        self.tick
    }

    pub fn sub_tick_time(&self) -> f32 {
        let now = SteadyTime::now();
        let duration = (now - self.previous_tick).num_nanoseconds().unwrap_or(0);
        (duration as f32 / self.average_tick_nanoseconds as f32).min(1.)
    }

    pub fn next_tick(&mut self) {
        let tick_duration = Duration::milliseconds(50);

        let mut now = SteadyTime::now();
        let mut duration = now - self.previous_tick;
        let wait = tick_duration - duration;
        if let Ok(wait_std) = wait.to_std() {
            thread::sleep(wait_std);
            duration = tick_duration;
            now = now + wait;
        }
        self.average_tick_nanoseconds *= AVERAGE_TICK_COUNT - 1;
        self.average_tick_nanoseconds += duration.num_nanoseconds().unwrap_or(i64::max_value()) as u64;
        self.average_tick_nanoseconds /= AVERAGE_TICK_COUNT;
        self.previous_tick = now;
        self.tick = self.tick.next();
    }
}
