use memory_stats::{memory_stats, MemoryStats};

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
                println!("{} {:.2}", self.tip, usage);
            }
        }
    }
}