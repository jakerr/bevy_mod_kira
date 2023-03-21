use bevy::{
    app::Plugin,
    prelude::{warn, AddAsset, Component, Handle},
    time::Timer,
};

use kira::{
    manager::{
        backend::cpal::CpalBackend, error::PlaySoundError, AudioManager, AudioManagerSettings,
    },
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
pub use static_sound_loader::{KiraStaticSoundAsset, StaticSoundFileLoader};

mod plugins;
mod static_sound_loader;

pub use plugins::*;

pub struct KiraPlugin;

impl Plugin for KiraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_non_send_resource::<KiraContext>()
            .add_asset::<KiraStaticSoundAsset>()
            .add_asset_loader(StaticSoundFileLoader)
            .add_plugin(plugins::KiraEventsPlugin);
        // .add_plugin(plugins::KiraDebugPlugin);
    }
}

// This is a non-send resource. If we were only targeting desktop we could use a normal resource
// wrapping a SyncCell since the AudioManager is sync on desktop but that's not true for every
// platform that we want to support i.e. Android and wasm.
pub struct KiraContext {
    manager: Option<AudioManager>,
}

#[derive(Component)]
pub struct KiraSoundHandle(pub Handle<KiraStaticSoundAsset>);

impl Default for KiraContext {
    fn default() -> Self {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default());
        if let Err(ref error) = manager {
            warn!("Error creating KiraContext: {}", error);
        }
        Self {
            manager: manager.ok(),
        }
    }
}

impl KiraContext {
    // Takes the same params as AudioManager::play calls the internal manager and then converts the handle into a bevy component type.
    pub fn play(
        &mut self,
        sound: StaticSoundData,
    ) -> Result<StaticSoundHandle, PlaySoundError<()>> {
        if let Some(manager) = &mut self.manager {
            manager.play(sound)
        } else {
            Err(PlaySoundError::IntoSoundError(()))
        }
    }

    pub fn get_manager(&mut self) -> Option<&mut AudioManager> {
        if let Some(manager) = &mut self.manager {
            return Some(manager);
        }
        None
    }
}

struct TimerMs<const N: i32> {
    timer: Timer,
}

impl<const N: i32> Default for TimerMs<N> {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(N as f32 / 1000.0, bevy::time::TimerMode::Repeating),
        }
    }
}
