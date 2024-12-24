use std::cell::RefCell;
use std::thread;
use std::time::Duration;
use tokio::time::Instant;

const NANOS_PER_MILLI: u32 = 1_000_000;
const NANOS_PER_FRAME: u64 = 16_666_666;

thread_local! {
    pub static  FRAME_RATE_CONTROLLER: RefCell<FrameRateController> = RefCell::new(FrameRateController::new());
}

pub fn next_frame() -> u64 {
    FRAME_RATE_CONTROLLER.with_borrow_mut(|frc| frc.next_frame())
}

pub fn get_total_frames() -> u64 {
    FRAME_RATE_CONTROLLER.with_borrow(|frc| frc.total_frames)
}

struct FrameRateController {
    start_time: Instant,
    current_frame: u64,
    total_frames: u64,
}

impl FrameRateController {
    fn new() -> FrameRateController {
        Self {
            start_time: Instant::now(),
            current_frame: 0,
            total_frames: 0,
        }
    }

    /// Advance frame and return time to wait as nanos.
    fn next_frame(&mut self) -> u64 {
        let now = Instant::now();
        let mut next_frame_no = now.duration_since(self.start_time).as_nanos() as u64 / NANOS_PER_FRAME + 1;
        if next_frame_no == self.current_frame {
            next_frame_no += 1;
        }
        self.total_frames += 1;
        let next_frame_time_nano = next_frame_no * NANOS_PER_FRAME;
        let now_nano = now.duration_since(self.start_time).as_nanos() as u64;
        next_frame_time_nano - now_nano
    }
}

#[test]
pub fn test_next_frame() {
    let mut controller = FrameRateController::new();
    let time = controller.next_frame();
    assert!(time > 0);
    assert!(time < NANOS_PER_FRAME * 2);
}