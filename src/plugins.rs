pub(crate) mod debug;
pub(crate) mod events;

use bevy::prelude::{AddAsset, Plugin};

use crate::KiraContext;
use events::*;

pub struct KiraPlugin;

impl Plugin for KiraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_non_send_resource::<KiraContext>()
            .add_asset::<KiraStaticSoundAsset>()
            .add_asset_loader(StaticSoundFileLoader)
            .add_plugins(KiraEventsPlugin);
        // .add_plugin(plugins::KiraDebugPlugin);
    }
}
