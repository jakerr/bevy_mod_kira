use bevy::prelude::Events;

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
            .add_system(do_play_sys)
            .add_system(cleanup_inactive_sounds_sys)
            .register_type::<KiraPlayingSounds>();
    }
}
