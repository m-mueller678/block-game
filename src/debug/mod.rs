use std::io::{stdin, BufRead};
use logging::root_logger;

mod triggers;

pub fn manager() -> &'static DebugManager {
    &GLOBAL_MANAGER
}

pub use self::triggers::{TriggerList, DebugTrigger};

lazy_static! {
    static ref GLOBAL_MANAGER:DebugManager=DebugManager::new();
}

#[derive(Debug)]
pub enum CommandError {
    Syntax(String),
    InvalidCommand,
    Custom(String),
}

pub struct DebugManager {
    pub triggers: TriggerList,
}

impl DebugManager {
    fn new() -> Self {
        use std::thread;
        thread::spawn(|| {
            let lock = stdin();
            let mut lock = lock.lock();
            let mut line = String::new();
            loop {
                match lock.read_line(&mut line) {
                    Ok(_) => {
                        match Self::run_command(&line) {
                            Ok(()) => {}
                            Err(CommandError::InvalidCommand) => {
                                println!("invalid command");
                            }
                            Err(CommandError::Syntax(s)) => {
                                println!("invalid syntax, expected:\n{}", s);
                            }
                            Err(CommandError::Custom(e)) => {
                                println!("error executing command: {}", e);
                            }
                        }
                    }
                    Err(_) => {
                        warn!(root_logger(), "cannot read from stdin, shutting down debug console");
                        break;
                    }
                }
                line.clear();
            }
        });
        DebugManager {
            triggers: Default::default(),
        }
    }

    pub fn run_command(cmd: &str) -> Result<(), CommandError> {
        let mut words = cmd.split_whitespace();
        match words.next() {
            Some("trigger") => {
                const SYNTAX: &str = "<trigger_name>";
                if let Some(t) = words.next() {
                    if words.next().is_some() {
                        return Err(CommandError::Syntax(SYNTAX.into()));
                    }
                    GLOBAL_MANAGER.triggers.trigger(t).map_err(|()| CommandError::Custom("unknown trigger name".into()))
                } else {
                    Err(CommandError::Syntax(SYNTAX.into()))
                }
            }
            Some(_) => Err(CommandError::InvalidCommand),
            None => { Ok(()) }
        }
    }
}