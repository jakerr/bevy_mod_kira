use bevy::{
    app::Plugin,
    prelude::{warn, AddAsset, Component, Handle, Resource},
    time::Timer,
    utils::synccell::SyncCell,
};

use kira::{
    manager::{
        backend::cpal::CpalBackend, error::PlaySoundError, AudioManager, AudioManagerSettings,
    },
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
pub use static_sound_loader::{StaticSoundAsset, StaticSoundFileLoader};

mod plugins;
mod static_sound_loader;

pub use plugins::*;

pub struct KiraPlugin;

impl Plugin for KiraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<KiraContext>()
            .add_asset::<StaticSoundAsset>()
            .add_asset_loader(StaticSoundFileLoader)
            .add_plugin(plugins::KiraEventsPlugin);
        // .add_plugin(plugins::KiraDebugPlugin);
    }
}

#[derive(Resource)]
pub struct KiraContext {
    manager: Option<SyncCell<AudioManager>>,
}

#[derive(Component)]
pub struct KiraSoundHandle(pub Handle<StaticSoundAsset>);

impl Default for KiraContext {
    fn default() -> Self {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default());
        if let Err(ref error) = manager {
            warn!("Error creating KiraContext: {}", error);
        }
        Self {
            manager: manager.ok().map(SyncCell::new),
        }
    }
}

impl KiraContext {
    pub fn with_manager<T>(&mut self, mut closure: T)
    where
        T: FnMut(&mut AudioManager),
    {
        if let Some(manager) = &mut self.manager {
            let exclusive_manager = manager.get();
            closure(exclusive_manager);
        }
    }

    // Takes the same params as AudioManager::play calls the internal manager and then converts the handle into a bevy component type.
    pub fn play(
        &mut self,
        sound: StaticSoundData,
    ) -> Result<StaticSoundHandle, PlaySoundError<()>> {
        if let Some(manager) = &mut self.manager {
            let exclusive_manager = manager.get();
            exclusive_manager.play(sound)
        } else {
            Err(PlaySoundError::IntoSoundError(()))
        }
    }

    pub fn get_manager(&mut self) -> Option<&mut AudioManager> {
        if let Some(manager) = &mut self.manager {
            let exclusive_manager = manager.get();
            return Some(exclusive_manager);
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
