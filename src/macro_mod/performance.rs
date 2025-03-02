#[macro_export]
macro_rules! warn_time {
    ($time: expr, $msg: expr) => {
        #[allow(unused_variables)]
        let time = $crate::performance::TimeLog::new(log::Level::Warn, $time, $msg);
    };
}
