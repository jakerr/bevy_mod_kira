use bevy::prelude::Events;

use bevy::app::Plugin;

pub use crate::static_sound_loader::{KiraStaticSoundAsset, StaticSoundFileLoader};
use kira::sound::{static_sound::StaticSoundData, SoundData};

mod clocks;
mod playback;
mod tracks;
pub use clocks::*;
pub use playback::*;
pub use tracks::*;

pub struct KiraEventsPlugin {
    plugins: Vec<Box<dyn Plugin>>,
}

impl KiraEventsPlugin {
    pub fn new() -> Self {
        Self { plugins: vec![] }
    }

    pub fn with_sound_data_source<D>(mut self) -> Self
    where
        D: SoundData + Send + Sync + Clone + 'static,
        D::Handle: Send + Sync + 'static,
    {
        self.plugins.push(Box::new(KiraSoundSourcePlugin::<D> {
            _marker: std::marker::PhantomData,
        }));
        self
    }
}

struct KiraSoundSourcePlugin<D>
where
    D: SoundData + Send + Sync + Clone + 'static,
    D::Handle: Send + Sync + 'static,
{
    _marker: std::marker::PhantomData<D>,
}

impl<D> Plugin for KiraSoundSourcePlugin<D>
where
    D: SoundData + Send + Sync + Clone + 'static,
    D::Handle: Send + Sync + 'static,
{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<KiraPlaySoundEvent<D>>()
            // The following events will not have automatic cleanup we need to manually consume them
            // to take the internal data out of the events.
            .add_system(do_play_sys::<D>);
    }
}

impl Plugin for KiraEventsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<KiraPlaySoundEvent<StaticSoundData>>()
            // The following events will not have automatic cleanup we need to manually consume them
            // to take the internal data out of the events.
            .init_resource::<Events<KiraAddTrackEvent>>()
            .init_resource::<Events<KiraAddClockEvent>>()
            .add_system(do_play_sys::<StaticSoundData>)
            .add_system(do_add_track_sys)
            .add_system(do_add_clock_sys)
            .add_system(cleanup_inactive_sounds_sys)
            .register_type::<KiraActiveSounds>();
        for plugin in self.plugins.iter() {
            plugin.build(app);
        }
    }
}
