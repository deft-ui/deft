use log::debug;

pub fn print_trace(msg: &str) {
    debug!("{}", msg);
    backtrace::trace(|frame| {
        // debug!("capture_and_print_backtrace frame={:?}", frame);
        backtrace::resolve_frame(frame, |symbol| {
            let file_name = symbol.filename().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            let file_no = symbol.lineno().unwrap_or(0);
            debug!("{}:{}", file_name, file_no);
        });
        true // keep going to the next frame
    });
}