use world::timekeeper::TickId;
use std::sync::{Arc, Mutex};
use std::cmp::{Ord, Ordering};
use vecmath::{vec3_scale, vec3_sub, vec3_add};

pub fn new() -> (PositionInterpolator, PositionUpdateSender) {
    let shared = Default::default();
    let s2 = Arc::clone(&shared);
    (
        PositionInterpolator {
            shared,
            update_tick: TickId::zero(),
            p0: [0.0; 3],
            p1: [0.0; 3],
            p2: [0.0; 3],
        },
        PositionUpdateSender {
            shared: s2
        }
    )
}

pub struct PositionUpdateSender {
    shared: Arc<Mutex<([f64; 3], TickId)>>,
}

impl PositionUpdateSender {
    pub fn send(&self, pos: [f64; 3], tick: TickId) {
        *(self.shared.lock().unwrap()) = (pos, tick);
    }
}

pub struct PositionInterpolator {
    update_tick: TickId,
    p0: [f64; 3],
    p1: [f64; 3],
    p2: [f64; 3],
    shared: Arc<Mutex<([f64; 3], TickId)>>,
}

impl PositionInterpolator {
    pub fn most_recent(&mut self) -> [f64; 3] {
        self.try_update();
        self.p0
    }

    fn try_update(&mut self) {
        let shared = self.shared.lock().unwrap();
        if self.update_tick < shared.1 {
            self.p2 = self.p1;
            self.p1 = self.p0;
            self.p0 = shared.0;
            self.update_tick = shared.1;
        }
    }

    pub fn pos(&mut self, tick: TickId, sub_tick: f32) -> [f64; 3] {
        self.try_update();
        match tick.cmp(&self.update_tick.next()) {
            Ordering::Less => {
                vec3_add(self.p2, vec3_scale(vec3_sub(self.p1, self.p2), sub_tick as f64))
            }
            Ordering::Equal => {
                vec3_add(self.p1, vec3_scale(vec3_sub(self.p0, self.p1), sub_tick as f64))
            }
            Ordering::Greater => {
                self.p0
            }
        }
    }
}