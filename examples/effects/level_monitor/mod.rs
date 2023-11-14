use std::collections::VecDeque;

use bevy::prelude::*;
use ringbuf::HeapProducer;

use kira::{
    clock::clock_info::ClockInfoProvider, dsp::Frame,
    modulator::value_provider::ModulatorValueProvider, track::effect::Effect,
};

mod builder;
mod handle;
pub use builder::LevelMonitorBuilder;
pub use handle::LevelMonitorHandle;

#[derive(Debug, Clone)]
pub struct LevelSample<const N: usize> {
    pub window: [Frame; N],
}

struct LevelMonitor<const N: usize> {
    sample_producer: HeapProducer<LevelSample<N>>,
    // Holds the last N frames.
    // They are only copied to the producer when the producer is empty.
    raw: VecDeque<Frame>,
}

impl<const N: usize> LevelMonitor<N> {
    fn new(sample_producer: HeapProducer<LevelSample<N>>) -> Self {
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

        if let Err(sample) = self.sample_producer.push(LevelSample { window }) {
            warn!(
                "LevelMonitor: Failed to send sample to consumer: {:?}",
                sample
            );
        }
    }
}

impl<const N: usize> Effect for LevelMonitor<N> {
    fn process(
        &mut self,
        input: Frame,
        _dt: f64,
        _clock_info_provider: &ClockInfoProvider,
        _modulator_value_provider: &ModulatorValueProvider,
    ) -> Frame {
        self.raw.push_back(input);
        if self.raw.len() > N {
            self.raw.pop_front();
        }
        self.send_sample();
        input
    }
}
