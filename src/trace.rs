pub fn print_trace() {
    backtrace::trace(|frame| {
        println!("capture_and_print_backtrace frame={:?}", frame);
        backtrace::resolve_frame(frame, |symbol| {
            println!("capture_and_print_backtrace resolve_frame={:?}", symbol);
        });
        true // keep going to the next frame
    });
}