use log::{Metadata, Record};

pub struct SimpleLogger {
    max_level: log::LevelFilter,
}

impl SimpleLogger {
    fn new() -> Self {
        SimpleLogger {
            max_level: log::LevelFilter::Info,
        }
    }

    pub fn init_with_max_level(max_level: log::LevelFilter) {
        let logger = SimpleLogger::new();
        if log::set_boxed_logger(Box::new(logger)).is_ok() {
            log::set_max_level(max_level);
        }
    }

}

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
