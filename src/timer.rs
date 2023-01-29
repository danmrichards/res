use spin_sleep::SpinSleeper;
use std::time::{Duration, Instant};

/// A timer that performs accurate waits.
pub struct Timer {
    start: Instant,
    sleeper: SpinSleeper,
}

impl Timer {
    /// Returns a new timer.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            sleeper: SpinSleeper::default(),
        }
    }

    /// Resets the timer.
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }

    /// Accurately waits for the given time.
    pub fn wait(&self, dur: Duration) {
        let elapsed = Instant::now() - self.start;
        if dur > elapsed {
            let wait_time = dur - elapsed;
            if wait_time.as_millis() > 1 {
                self.sleeper.sleep(wait_time);
            }
        }
    }
}
