use anyhow::{anyhow, Error};
use kira::manager::backend::cpal::CpalBackend;
use kira::manager::AudioManager;
use kira::sound::static_sound::StaticSoundHandle;
use kira::sound::PlaybackState;
use kira::sound::SoundData;
use std::any::Any;

/// KiraPlayable is a trait that allows KiraPlugin to play static (sounds loaded from a supported
/// sound file) and dynamic sounds implementations of `kira::sound::Sound`.
///
/// Static sounds loaded by the asset loader implement this type automatically through a blanket
/// implementation.
///
/// In order to make a custom dynamic sound that is also KiraPlayable three type implementations are required:
///  1. A type that implements kira::sound::SoundData where the associated handle type is
///     a [`DynamicSoundHandle`].
///  2. The handle type that implements [`DynamicSoundHandle`].
///  3. The sound type that implements `kira::sound::Sound`.
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

/// A trait that allows communication with a dynamic sound that is currently playing.
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
        let res = manager.play::<D>(self.clone());
        res.map_err(|_e| anyhow!("failed to play sound: {}", std::any::type_name::<D>()))
            .map(|handle| handle.into())
    }
}
