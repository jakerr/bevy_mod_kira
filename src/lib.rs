use bevy::{
    app::Plugin,
    prelude::{error, AddAsset, Component, Handle},
    time::Timer,
};
use std::{
    any::Any,
    f64::consts::PI,
    fmt::{Debug, Display},
};

use kira::{
    dsp::Frame,
    manager::{
        backend::cpal::CpalBackend, error::PlaySoundError, AudioManager, AudioManagerSettings,
    },
    sound::{
        static_sound::{StaticSoundData, StaticSoundHandle},
        Sound, SoundData,
    },
};
pub use static_sound_loader::{KiraStaticSoundAsset, StaticSoundFileLoader};

mod plugins;
mod static_sound_loader;

pub use plugins::*;
pub use static_sound_loader::KiraSoundSource;

pub struct KiraPlugin;

#[derive(Clone)]
pub struct MySoundData;
struct MySound(f64);
pub struct MySoundHandle;

impl Sound for MySound {
    fn track(&mut self) -> kira::track::TrackId {
        kira::track::TrackId::Main
    }

    fn process(
        &mut self,
        dt: f64,
        _clock_info_provider: &kira::clock::clock_info::ClockInfoProvider,
    ) -> kira::dsp::Frame {
        self.0 += dt;
        let middle_c = 261.626;
        let tone = (self.0 * middle_c * 2.0 * PI).sin() as f32;
        let progress = self.0 / 10.0;
        let scaled = 0.6 * tone * (progress * PI).sin() as f32;
        Frame {
            left: scaled,
            right: scaled,
        }
    }

    fn finished(&self) -> bool {
        self.0 > 10.0
    }
}

impl SoundData for MySoundData {
    type Handle = MySoundHandle;
    type Error = ();

    fn into_sound(self) -> Result<(Box<dyn kira::sound::Sound>, Self::Handle), Self::Error> {
        Ok((Box::new(MySound(0.0)), MySoundHandle))
    }
}

impl Plugin for KiraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_non_send_resource::<KiraContext>()
            .add_asset::<KiraStaticSoundAsset>()
            .add_asset_loader(StaticSoundFileLoader)
            .add_plugin(plugins::KiraEventsPlugin::new().with_sound_data_source::<MySoundData>());
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
pub struct KiraStaticSoundHandle(pub Handle<KiraStaticSoundAsset>);

#[derive(Component)]
pub struct KiraDynamicSoundHandle<T: SoundData + Clone>(pub KiraSoundSource<T>);

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

pub struct KiraEventError<D: SoundData> {
    message: String,
    // Because sound data Error type is not constrained to std::error::Error we'll just store the
    // type name.
    //
    // Todo: try making this cause a PlaySoundError instead of a generic error and see if that makes
    // this easier.
    cause: Option<D::Error>,
}

impl<D: SoundData> KiraEventError<D> {
    pub fn new(message: impl Into<String>, cause: Option<D::Error>) -> Self {
        Self {
            message: message.into(),
            cause,
        }
    }
}

impl<D: SoundData> std::error::Error for KiraEventError<D> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // We can't extract a cause because the Error type on the SoundData trait insn't constrained
        // to std::error::Error.
        None
    }
}

impl<D: SoundData> Display for KiraEventError<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error sending event to kira: {}. for sound type: {}, with error type: {}",
            self.message,
            std::any::type_name::<D>(),
            if self.cause.is_some() {
                std::any::type_name::<D::Error>()
            } else {
                "None"
            },
        )
    }
}

impl<D: SoundData> Debug for KiraEventError<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KiraEventError: {}", self)
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

    pub fn play_d<D>(&mut self, sound: D) -> Result<D::Handle, KiraEventError<D>>
    where
        D: SoundData,
    {
        if let Some(manager) = &mut self.manager {
            return manager.play(sound).map_err(|err| match err {
                PlaySoundError::SoundLimitReached => {
                    KiraEventError::new("sound limit reached", None)
                }
                PlaySoundError::IntoSoundError(e) => {
                    KiraEventError::new("into sound error", Some(e))
                }
                PlaySoundError::CommandError(_) => KiraEventError::new("command error", None),
                _ => KiraEventError::new("unknown error", None),
            });
        }
        return Err(KiraEventError::new("no manager", None));
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
