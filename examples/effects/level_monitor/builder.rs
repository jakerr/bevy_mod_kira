use ringbuf::HeapRb;

use kira::{
    track::effect::{Effect, EffectBuilder},
    Volume,
};

use super::{LevelMonitor, LevelMonitorHandle};

const SAMPLE_CAPACITY: usize = 4;

/// Configures a volume control effect.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LevelMonitorBuilder(pub Volume);

impl LevelMonitorBuilder {
    /// Creates a new [`LevelMonitorBuilder`].
    pub fn new(volume: impl Into<Volume>) -> Self {
        Self(volume.into())
    }
}

impl Default for LevelMonitorBuilder {
    fn default() -> Self {
        Self(Volume::Amplitude(1.0))
    }
}

impl EffectBuilder for LevelMonitorBuilder {
    type Handle = LevelMonitorHandle;

    fn build(self) -> (Box<dyn Effect>, Self::Handle) {
        let (sample_producer, sample_consumer) = HeapRb::new(SAMPLE_CAPACITY).split();
        (
            Box::new(LevelMonitor::new(self, sample_producer)),
            LevelMonitorHandle { sample_consumer },
        )
    }
}
