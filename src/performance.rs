use log::{log, Level};
use std::time::Instant;

pub struct TimeLog {
    message: String,
    start: Instant,
    time: u64,
    level: log::Level,
}

impl Drop for TimeLog {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_millis() as u64;
        if elapsed > self.time {
            log!(self.level, "{} took {}ms", self.message, elapsed);
        }
    }
}

impl TimeLog {
    pub fn new(level: Level, time: u64, msg: &str) -> Self {
        Self {
            start: Instant::now(),
            time,
            level,
            message: msg.to_string(),
        }
    }
}
