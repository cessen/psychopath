#![allow(dead_code)]

use time;
use std::time::Duration;
use std::thread;

#[derive(Copy, Clone)]
pub struct Timer {
    last_time: u64,
}

impl Timer {
    pub fn new() -> Timer {
        Timer { last_time: time::precise_time_ns() }
    }

    /// Marks a new tick time and returns the time elapsed in seconds since
    /// the last call to tick().
    pub fn tick(&mut self) -> f32 {
        let n = time::precise_time_ns();
        let dt = n - self.last_time;
        self.last_time = n;

        dt as f32 / 1000000000.0
    }

    /// Returns the time elapsed in seconds since the last call to tick().
    pub fn elapsed(&self) -> f32 {
        let dt = time::precise_time_ns() - self.last_time;
        dt as f32 / 1000000000.0
    }

    /// Sleeps the current thread until n seconds after the last tick.
    pub fn sleep_until(&self, n: f32) {
        let dt = time::precise_time_ns() - self.last_time;
        let target_dt = ((n as f64) * 1000000000.0) as u64;
        if dt < target_dt {
            let delay = target_dt - dt;
            let seconds = delay / 1000000000;
            let nanoseconds = delay % 1000000000;
            thread::sleep(Duration::new(seconds, nanoseconds as u32));
        }
    }
}
