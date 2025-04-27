use std::collections::VecDeque;

use bevy::prelude::*;
use ringbuf::{
    HeapProd,
    traits::{Observer, *},
};

use kira::{Frame, effect::Effect};

mod builder;
mod handle;
pub use builder::LevelMonitorBuilder;
pub use handle::LevelMonitorHandle;

#[derive(Debug, Clone)]
pub struct LevelSample<const N: usize> {
    pub window: [Frame; N],
}

struct LevelMonitor<const N: usize> {
    sample_producer: HeapProd<LevelSample<N>>,
    // Holds the last N frames.
    // They are only copied to the producer when the producer is empty.
    raw: VecDeque<Frame>,
}

// Ringbuf is a lock free producer, so we can use it in the audio thread.
unsafe impl<const N: usize> Sync for LevelMonitor<N> {}

impl<const N: usize> LevelMonitor<N> {
    fn new(sample_producer: HeapProd<LevelSample<N>>) -> Self {
        Self {
            sample_producer,
            raw: VecDeque::new(),
        }
    }

    fn send_sample(&mut self) {
        if self.sample_producer.is_full() || self.raw.len() < N {
            return;
        }

        let mut window = [Frame::ZERO; N];
        let samples = self.raw.make_contiguous();
        window.clone_from_slice(samples);
        // for (i, frame) in self.raw.iter().enumerate() {
        //     window[i] = *frame;
        // }
        // let sample = LevelSample { window };

        if let Err(sample) = self.sample_producer.try_push(LevelSample { window }) {
            warn!(
                "LevelMonitor: Failed to send sample to consumer: {:?}",
                sample
            );
        }
    }
}

impl<const N: usize> Effect for LevelMonitor<N> {
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &kira::info::Info) {
        for frame in input.iter_mut() {
            self.raw.push_back(*frame);
            if self.raw.len() > N {
                self.raw.pop_front();
            }
            self.send_sample();
        }
    }
}
