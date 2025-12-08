use std::time::{Duration, Instant};

pub struct LoadingAnimation {
    frames: Vec<&'static str>,
    current_frame: usize,
    last_update: Instant,
    frame_duration: Duration,
}

impl LoadingAnimation {
    pub fn new() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current_frame: 0,
            last_update: Instant::now(),
            frame_duration: Duration::from_millis(80),
        }
    }

    pub fn tick(&mut self) {
        if self.last_update.elapsed() >= self.frame_duration {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
    }

    pub fn current_frame(&self) -> &'static str {
        self.frames[self.current_frame]
    }
}

impl Default for LoadingAnimation {
    fn default() -> Self {
        Self::new()
    }
}
