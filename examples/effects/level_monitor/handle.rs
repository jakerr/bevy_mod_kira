use super::LevelSample;
use anyhow::Error;
use ringbuf::{HeapCons, traits::*};

// Receives samples from the audio thread in chunks of N frames.
pub struct LevelMonitorHandle<const N: usize> {
    pub(super) sample_consumer: HeapCons<LevelSample<N>>,
}

impl<const N: usize> LevelMonitorHandle<N> {
    pub fn get_sample(&mut self) -> Result<LevelSample<N>, Error> {
        if let Some(sample) = self.sample_consumer.try_pop() {
            Ok(sample)
        } else {
            Err(Error::msg("No sample available"))
        }
    }
}
