pub(crate) mod debug;
pub(crate) mod events;

use bevy::{asset::AssetApp, prelude::Plugin};

use crate::KiraContext;
use events::*;

pub struct KiraPlugin;

impl Plugin for KiraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_non_send_resource::<KiraContext>()
            .register_asset_loader(StaticSoundFileLoader)
            .init_asset::<KiraStaticSoundAsset>()
            .add_plugins(KiraEventsPlugin);
        // .add_plugin(plugins::KiraDebugPlugin);
    }
}
