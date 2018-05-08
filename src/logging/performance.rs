use slog::Logger;
use logging::root_logger;
use time::precise_time_ns;

#[cfg(debug_assertions)]
const ENABLE_PERFORMANCE_LOGGING: bool = false;
#[cfg(not(debug_assertions))]
const ENABLE_PERFORMANCE_LOGGING: bool = true;

struct ActionEntry {
    pub name: String,
    pub max_time: u64,
    pub max_average: u64,
    pub average: u64,
}

const TIMING_MULTIPLIER: u64 = 256;

pub struct Monitor {
    total: ActionEntry,
    actions: Vec<ActionEntry>,
    logger: Logger,
    actions_completed: Option<usize>,
    last_submit: u64,
    run_time: u64,
}

impl Monitor {
    pub fn new(name: String, max_time: u64, max_average: u64, actions: &[(&str, u64, u64)]) -> Self {
        Monitor {
            logger: root_logger().new(o!("performance monitor name"=>name.clone())),
            actions: actions.iter()
                .map(|&(ref n, t, a)| ActionEntry
                    {
                        name: name.clone() + ">" + n,
                        max_time: t,
                        max_average: a * TIMING_MULTIPLIER,
                        average: 0,
                    }
                ).collect(),
            total: ActionEntry {
                name,
                max_time: max_time,
                max_average: max_average * TIMING_MULTIPLIER,
                average: 0,
            },
            actions_completed: None,
            last_submit: 0,
            run_time: 0,
        }
    }

    pub fn start_run(&mut self) {
        assert!(self.actions_completed.is_none());
        self.actions_completed = Some(0);
        self.last_submit = precise_time_ns();
        self.run_time = 0;
    }

    pub fn action_complete(&mut self) {
        let old_time = self.last_submit;
        self.last_submit = precise_time_ns();
        let dt = self.last_submit - old_time;
        self.submit_timing(dt);
    }

    pub fn end_run(&mut self) {
        assert_eq!(self.actions_completed, Some(self.actions.len()));
        self.actions_completed = None;
        Self::submit_to_action(&mut self.total, self.run_time, &self.logger);
    }

    fn submit_timing(&mut self, duration_ns: u64) {
        if let Some(ref mut action_index) = self.actions_completed {
            Self::submit_to_action(&mut self.actions[*action_index], duration_ns, &self.logger);
            *action_index += 1;
            assert!(*action_index <= self.actions.len());
            self.run_time += duration_ns;
        } else {
            panic!("Monitor state error");
        }
    }

    fn submit_to_action(action: &mut ActionEntry, duration_ns: u64, logger: &Logger) {
        if !ENABLE_PERFORMANCE_LOGGING {
            return;
        }
        {
            let ref mut average = action.average;
            *average = *average * TIMING_MULTIPLIER - *average;
            *average /= TIMING_MULTIPLIER;
            *average += duration_ns;
        }
        if action.average > action.max_average {
            warn!(logger, "action timing average";
            "actual_average" => action.average / TIMING_MULTIPLIER,
            "max_average" => action.max_average / TIMING_MULTIPLIER,
            "action_name" => &action.name,
            )
        }
        if duration_ns > action.max_time {
            warn!(logger, "single action timing";
            "actual_time" => duration_ns,
            "max_time" => action.max_time,
            "action_name" => &action.name,
            )
        }
    }
}
