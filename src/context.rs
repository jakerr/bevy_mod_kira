use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use anyhow::{Error, anyhow};
use bevy::prelude::*;

use crate::sound::sound_types::{KiraPlayable, KiraPlayingSound, KiraTrackHandle};
use kira::{
    AudioManager, AudioManagerSettings,
    backend::cpal::CpalBackend,
    clock::{ClockHandle, ClockSpeed},
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    track::{TrackBuilder, TrackHandle},
};
use std::ops::DerefMut;

/// KiraContext is a non-send resource that provides access to an initialized `kira::AudioManager`.
/// Storing this in a non-send resource is necessary in order to support environments such as web
/// (WebAssembly) and Android where kira's AudioManager is non-sync. For simplicity's sake the
/// context is stored in a non-send resource everywhere.
///
/// In practice this means that systems that use the context will need to take
/// a `NonSendMut<KiraContext>` as a parameter which will instruct Bevy to run the system on the
/// main thread. For this reason it is recommended to only interface directly through the context
/// for setup systems for example to create tracks and clocks. (See the drum_machine example.)
///
/// For systems that want to trigger sound playback they should send a [`KiraPlaySoundEvent`] via a
/// `EventWriter<PlaySoundEvent>` which is a thread-safe event channel so does not impact
/// the parallelizability of the system.
///
/// [`KiraPlaySoundEvent`]: crate::plugins::events::KiraPlaySoundEvent
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
    pub fn play(
        &mut self,
        sound: Box<dyn KiraPlayable>,
        track: Option<&mut KiraTrackHandle>,
    ) -> Result<KiraPlayingSound, Error> {
        let manager = self.get_manager()?;
        match track {
            Some(track) => sound.play_in_track(track).map_err(|e| e.into()),
            None => {
                let main_track = manager.main_track();
                sound.play_in_main_track(main_track).map_err(|e| e.into())
            }
        }
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
