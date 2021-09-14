use std::{
    collections::hash_map::DefaultHasher,
    convert::TryInto,
    fs::File,
    hash::{Hash, Hasher},
    io::Write,
    path::Path,
    sync::Mutex,
    thread,
};

use anyhow::{Context, Result};
use log::{Level, LevelFilter, Metadata, Record};

struct SimpleLogger<W: Write + Sync + Send> {
    level: Option<Level>,
    log_file: Mutex<W>,
}

impl<W> SimpleLogger<W>
where
    W: Write + Sync + Send,
{
    fn new(level: LevelFilter, log_file: W) -> Self {
        Self {
            level: level.to_level(),
            log_file: Mutex::new(log_file),
        }
    }
}

impl<W> log::Log for SimpleLogger<W>
where
    W: Write + Sync + Send,
{
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if let Some(level) = self.level {
            if record.metadata().level() <= level {
                println!("{}", record.args());
            }
        }
        // always log to log-file, regardless of setting
        let thread_id = get_pseudo_thread_id();
        let mut log_file = self.log_file.lock().unwrap();
        let err = write!(&mut log_file, "[{}] {}\r\n", thread_id, record.args());
        if let Err(err) = err {
            println!("error writing to log-file: {}", err);
        }
    }

    fn flush(&self) {
        let err = self.log_file.lock().unwrap().flush();
        if let Err(err) = err {
            println!("error flushing log-file: {}", err);
        }
    }
}

/// Initializes the logging feature with the given log-level.
pub fn init<P: AsRef<Path>>(level: log::LevelFilter, log_file_path: P) -> Result<()> {
    let file = File::create(log_file_path).context("could not create log file")?;
    let logger = SimpleLogger::new(level, file);
    log::set_boxed_logger(Box::new(logger)).context("could not set logger")?;
    // Note: the logger implementation logs everything into the log-file
    // this means the optimization must be turned off
    log::set_max_level(log::LevelFilter::Trace);
    Ok(())
}

/// Gets a pseudo thread is that is unrelated to any os specific ID.
/// Only guarantee is that each number is unique given the the same thread calls this function.
fn get_pseudo_thread_id() -> u32 {
    let mut hasher = DefaultHasher::new();
    thread::current().id().hash(&mut hasher);
    let thread_id = hasher.finish();
    // thread_id is a hash that is somewhat random,
    // so the chance of a collision is slim when removing the lower 32 bits
    (thread_id >> 32).try_into().unwrap()
}
