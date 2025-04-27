use kira::effect::{Effect, EffectBuilder};
use ringbuf::{HeapRb, traits::*};

use super::{LevelMonitor, LevelMonitorHandle};

// Keeping this capacity low means that the sample will be updated less frequently as the audio
// thread will skip copying the samples into the ring buffer if it's not empty.
const SAMPLE_CAPACITY: usize = 1;

/// Configures a volume control effect.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LevelMonitorBuilder<const N: usize>;

impl<const N: usize> EffectBuilder for LevelMonitorBuilder<N> {
    type Handle = LevelMonitorHandle<N>;

    fn build(self) -> (Box<dyn Effect>, Self::Handle) {
        let (sample_producer, sample_consumer) = HeapRb::new(SAMPLE_CAPACITY).split();
        (
            Box::new(LevelMonitor::new(sample_producer)),
            LevelMonitorHandle { sample_consumer },
        )
    }
}
