use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const WINDOW: usize = 60;

#[derive(Debug, Default)]
pub struct FrameStats {
    samples: VecDeque<Duration>,
    time: Option<Instant>,
}

impl FrameStats {
    pub fn add_sample(&mut self) {
        if let Some(t) = self.time {
            while self.samples.len() >= WINDOW {
                self.samples.pop_front();
            }
            self.samples.push_back(t.elapsed());
        }
        self.time = Some(Instant::now());
    }

    pub fn calc_fps(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sum: u128 = 0;
        let mut count = 0f64;
        for s in self.samples.iter().map(|v| v.as_micros()) {
            sum += s;
            count += 1.0;
        }
        let avg_frame_micros = (sum as f64) / count;
        if avg_frame_micros == 0.0 {
            return 0.0;
        }
        (1_000_000.0 / avg_frame_micros) as f32
    }

    pub fn last_frame_millis(&self) -> Duration {
        if self.samples.is_empty() {
            Duration::ZERO
        } else {
            self.samples[self.samples.len() - 1]
        }
    }
}
