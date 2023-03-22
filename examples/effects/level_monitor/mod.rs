use std::collections::VecDeque;

use bevy::{log::Level, prelude::warn};
use ringbuf::{HeapConsumer, HeapProducer};

use kira::{clock::clock_info::ClockInfoProvider, dsp::Frame, track::effect::Effect};

mod builder;
mod handle;
pub use builder::LevelMonitorBuilder;
pub use handle::LevelMonitorHandle;

#[derive(Debug, Clone)]
pub struct LevelSample<const N: usize> {
    window: [Frame; N],
}

struct LevelMonitor<const N: usize> {
    sample_producer: HeapProducer<LevelSample<N>>,
    raw: VecDeque<Frame>,
}

impl<const N: usize> LevelMonitor<N> {
    fn new(builder: LevelMonitorBuilder, sample_producer: HeapProducer<LevelSample<N>>) -> Self {
        Self {
            sample_producer,
            raw: VecDeque::new(),
        }
    }

    fn send_sample(&mut self) {
        if self.sample_producer.is_full() {
            return;
        }
        let mut left_latest: f64 = 0.0;
        let mut right_latest: f64 = 0.0;
        let mut left_peak: f64 = 0.0;
        let mut right_peak: f64 = 0.0;

        let mut count = 0usize;
        let len = self.raw.len();
        let recent_window = (len - 10).min(0);
        for frame in self.raw.iter() {
            let left = frame.left.abs() as f64;
            let right = frame.right.abs() as f64;

            left_peak = left_peak.max(left);
            right_peak = right_peak.max(right);
            if count > recent_window {
                left_latest = left_latest.max(left);
                right_latest = right_latest.max(right);
            }
            count += 1;
        }

        let sample = LevelSample {
            left: left_latest,
            right: right_latest,
            left_peak,
            right_peak,
        };

        if let Err(sample) = self.sample_producer.push(sample) {
            warn!(
                "LevelMonitor: Failed to send sample to consumer: {:?}",
                sample
            );
        }
    }
}

impl Effect for LevelMonitor {
    fn process(&mut self, input: Frame, dt: f64, clock_info_provider: &ClockInfoProvider) -> Frame {
        self.raw.push_back(input);
        if self.raw.len() > 2048 {
            self.raw.pop_front();
        }
        self.send_sample();
        input
    }
}
