use ringbuf::{HeapCons, traits::*};

use kira::CommandError;

use super::LevelSample;

// Receives samples from the audio thread in chunks of N frames.
pub struct LevelMonitorHandle<const N: usize> {
    pub(super) sample_consumer: HeapCons<LevelSample<N>>,
}

impl<const N: usize> LevelMonitorHandle<N> {
    pub fn get_sample(&mut self) -> Result<LevelSample<N>, CommandError> {
        if let Some(sample) = self.sample_consumer.try_pop() {
            Ok(sample)
        } else {
            Err(CommandError::CommandQueueFull)
        }
    }
}
