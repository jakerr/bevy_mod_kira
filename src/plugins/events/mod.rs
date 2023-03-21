use bevy::prelude::Events;

use bevy::app::Plugin;

pub use crate::static_sound_loader::{KiraStaticSoundAsset, StaticSoundFileLoader};
use kira::sound::static_sound::StaticSoundData;

mod clocks;
mod playback;
mod tracks;
pub use clocks::*;
pub use playback::*;
pub use tracks::*;

pub struct KiraEventsPlugin;

impl Plugin for KiraEventsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<KiraPlaySoundEvent<StaticSoundData>>()
            // The following events will not have automatic cleanup we need to manually consume them
            // to take the internal data out of the events.
            .init_resource::<Events<KiraAddTrackEvent>>()
            .init_resource::<Events<KiraAddClockEvent>>()
            .add_system(do_play_sys)
            .add_system(do_add_track_sys)
            .add_system(do_add_clock_sys)
            .add_system(cleanup_inactive_sounds_sys)
            .register_type::<KiraActiveSounds>();
    }
}
