use bevy::prelude::{Events, Update};

use bevy::app::Plugin;

pub use crate::sound::static_sounds::{KiraStaticSoundAsset, StaticSoundFileLoader};

mod playback;
pub use playback::*;

pub struct KiraEventsPlugin;

impl Plugin for KiraEventsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // The following events will not have automatic cleanup we need to manually consume them
        // to take the internal data out of the events.
        app.init_resource::<Events<KiraPlaySoundEvent>>()
            .add_systems(Update, (do_play_sys, cleanup_inactive_sounds_sys))
            .register_type::<KiraPlayingSounds>();
    }
}
