use bevy::prelude::Events;

use bevy::app::Plugin;

pub use crate::static_sound_loader::{StaticSoundAsset, StaticSoundFileLoader};
use kira::sound::static_sound::StaticSoundData;

mod playback;
mod tracks;
pub use playback::*;
pub use tracks::*;

pub struct KiraEventsPlugin;

impl Plugin for KiraEventsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<PlaySoundEvent<StaticSoundData>>()
            // Track add events will not have automatic cleanup we need to manually consume them to
            // take the internal track out of the event.
            .init_resource::<Events<AddTrackEvent>>()
            .add_system(do_play_sys)
            .add_system(do_add_track_sys)
            .add_system(cleanup_inactive_sounds_sys)
            .register_type::<KiraActiveSounds>();
    }
}
