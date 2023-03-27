use std::any::Any;

use anyhow::{anyhow, Error};
use bevy::{
    app::Plugin,
    prelude::{error, AddAsset, Component, Handle},
    time::Timer,
};

use kira::{
    manager::{
        backend::cpal::CpalBackend, error::PlaySoundError, AudioManager, AudioManagerSettings,
    },
    sound::{
        static_sound::{PlaybackState, StaticSoundData, StaticSoundHandle},
        SoundData,
    },
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

pub trait KiraPlayable: Send + Sync + 'static {
    fn play_in_manager(
        &self,
        manager: &mut AudioManager<CpalBackend>,
    ) -> Result<KiraPlayingSound, Error>;
}

pub trait Downcastable: Any + Send + Sync {
    fn as_any(&self) -> &(dyn Any + Send + Sync);
}

impl<T: Any + Send + Sync> Downcastable for T {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}

pub trait DynamicSoundHandle: Downcastable {
    /// Returns the current playback state of the sound. This is used by bevy_mod_kira to determine
    /// if the sound is still playing. The only hard requirement is that this method returns
    /// `PlaybackState::Stopped` if a sound is finished and ready to be cleaned up else a non
    /// Stopped state should be returned.
    fn state(&self) -> PlaybackState;
}

pub enum KiraPlayingSound {
    Static(StaticSoundHandle),
    Dynamic(Box<dyn DynamicSoundHandle>),
}

impl From<StaticSoundHandle> for KiraPlayingSound {
    fn from(handle: StaticSoundHandle) -> Self {
        KiraPlayingSound::Static(handle)
    }
}

impl<D> From<D> for KiraPlayingSound
where
    D: DynamicSoundHandle,
{
    fn from(handle: D) -> Self {
        KiraPlayingSound::Dynamic(Box::new(handle))
    }
}

impl<D: SoundData> KiraPlayable for D
where
    D: Send + Sync + Clone + 'static,
    D::Handle: Into<KiraPlayingSound>,
{
    fn play_in_manager(
        &self,
        manager: &mut AudioManager<CpalBackend>,
    ) -> Result<KiraPlayingSound, Error> {
        // Result<DynHandle, Error> {
        let res = manager.play::<D>(self.clone());
        res.map_err(|_e| anyhow!("failed to play sound: {}", std::any::type_name::<D>()))
            .map(|handle| handle.into())
    }
}

// This is a non-send resource. If we were only targeting desktop we could use a normal resource
// wrapping a SyncCell since the AudioManager is sync on desktop but that's not true for every
// platform that we want to support i.e. Android and wasm.
pub struct KiraContext {
    manager: Option<AudioManager>,
}

#[derive(Component)]
pub struct KiraStaticSoundHandle(pub Handle<KiraStaticSoundAsset>);

impl Default for KiraContext {
    fn default() -> Self {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default());
        if let Err(ref error) = manager {
            error!("Error creating KiraContext: {}", error);
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

    pub fn play_dynamic(
        &mut self,
        sound: Box<dyn KiraPlayable>,
    ) -> Result<KiraPlayingSound, Error> {
        if let Some(manager) = &mut self.manager {
            sound.play_in_manager(manager)
        } else {
            Err(anyhow!("KiraContext has no manager"))
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
