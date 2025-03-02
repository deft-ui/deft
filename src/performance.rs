use log::{debug, error, log, Level, Log};
use memory_stats::{memory_stats, MemoryStats};
use tokio::time::Instant;

pub struct MemoryUsage {
    tip: String,
    start_memory_stats: Option<MemoryStats>,
}

impl MemoryUsage {
    pub fn new(tip: &str) -> Self {
        let tip = tip.to_string();
        Self {
            tip,
            start_memory_stats: memory_stats(),
        }
    }
}

impl Drop for MemoryUsage {
    fn drop(&mut self) {
        if let Some(ms) = memory_stats() {
            if let Some(begin) = self.start_memory_stats {
                let usage = (ms.physical_mem - begin.physical_mem) as f32 / 1024.0 / 1024.0;
                debug!("{} {:.2}", self.tip, usage);
            }
        }
    }
}

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
            log!(self.level, "{} took {}ms",self.message, elapsed);
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