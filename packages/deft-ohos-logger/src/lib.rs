pub use env_logger::fmt::Formatter;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use ohos_hilog_binding::hilog_info;
use std::fmt;

#[derive(Debug)]
pub struct OhosLogger {}

impl Log for OhosLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        //TODO match level
        let msg = fmt::format(format_args!("[{}] {}", record.level(), record.args()));
        hilog_info!("deft: {}", msg);
    }

    fn flush(&self) {}
}

static LOGGER: OhosLogger = OhosLogger {};

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}

