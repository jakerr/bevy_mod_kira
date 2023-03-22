use ringbuf::{HeapConsumer, HeapProducer};

use kira::{tween::Tween, CommandError, Volume};

use super::LevelSample;

/// Controls a volume control effect.
pub struct LevelMonitorHandle {
    pub(super) sample_consumer: HeapConsumer<LevelSample>,
}

impl LevelMonitorHandle {
    /// Sets the volume adjustment to apply to input audio.
    pub fn get_sample(&mut self) -> Result<LevelSample, CommandError> {
        if let Some(sample) = self.sample_consumer.pop() {
            Ok(sample)
        } else {
            Err(CommandError::CommandQueueFull)
        }
    }
}
