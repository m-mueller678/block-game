use std::sync::Mutex;
use slog_term::*;
use slog::Drain;

pub use slog::Logger;

pub fn root_logger()->&'static Logger{
    &ROOT_LOGGER
}

lazy_static!{
    static ref ROOT_LOGGER:Logger=create_logger();
}

fn create_logger() -> Logger {
    let decorator = TermDecorator::new().build();
    let drain = CompactFormat::new(decorator).build();
    let drain = Mutex::new(drain).fuse();
    let log = Logger::root(drain, o!());

    info!(&log, "logger initialized");
    log
}