use anyhow::{anyhow, Error};
use bevy::prelude::error;

use crate::sound::sound_types::{KiraPlayable, KiraPlayingSound};
pub use crate::sound::static_sounds::{KiraStaticSoundAsset, StaticSoundFileLoader};
use kira::{
    clock::ClockHandle,
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    track::{TrackBuilder, TrackHandle},
    ClockSpeed,
};

// This is a non-send resource. If we were only targeting desktop we could use a normal resource
// wrapping a SyncCell since the AudioManager is sync on desktop but that's not true for every
// platform that we want to support i.e. Android and wasm.
pub struct KiraContext {
    manager: Option<AudioManager>,
}

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
    pub fn play(&mut self, sound: StaticSoundData) -> Result<StaticSoundHandle, Error> {
        let manager = self.get_manager()?;
        manager.play(sound).map_err(|e| e.into())
    }

    pub fn play_dynamic(
        &mut self,
        sound: Box<dyn KiraPlayable>,
    ) -> Result<KiraPlayingSound, Error> {
        let manager = self.get_manager()?;
        sound.play_in_manager(manager)
    }

    pub fn add_clock(&mut self, clock_speed: ClockSpeed) -> Result<ClockHandle, Error> {
        let manager = self.get_manager()?;
        manager.add_clock(clock_speed).map_err(|e| e.into())
    }

    pub fn add_track(&mut self, track: TrackBuilder) -> Result<TrackHandle, Error> {
        let manager = self.get_manager()?;
        manager.add_sub_track(track).map_err(|e| e.into())
    }

    pub fn get_manager(&mut self) -> Result<&mut AudioManager, Error> {
        if let Some(manager) = &mut self.manager {
            return Ok(manager);
        }
        Err(anyhow!("KiraContext has no manager"))
    }
}
