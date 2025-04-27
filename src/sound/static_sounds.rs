use anyhow::Result;
use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetLoader, LoadContext};
use bevy::prelude::{Component, Handle, debug};
use bevy::reflect::TypePath;
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::sound::{FromFileError, SoundData};
use std::io::Cursor;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KiraError {
    #[error("An error occurred while reading the file from the filesystem")]
    IoError(#[from] std::io::Error),
    #[error("An error occurred when parsing the file")]
    FromFileError(#[from] FromFileError),
}

#[derive(TypePath, Clone, Asset)]
pub struct SoundAsset<T>
where
    T: TypePath + Send + Sync + SoundData + Clone,
{
    pub sound: T,
}

#[derive(Clone, TypePath)]
pub struct KiraStaticSoundData(pub StaticSoundData);

impl SoundData for KiraStaticSoundData {
    type Error = <StaticSoundData as SoundData>::Error;
    type Handle = <StaticSoundData as SoundData>::Handle;
    fn into_sound(
        self,
    ) -> std::result::Result<(Box<dyn kira::sound::Sound>, Self::Handle), Self::Error> {
        self.0.into_sound()
    }
}

// impl TypePath for KiraStaticSoundData {
//     fn type_path() -> &'static str {
//         "bevy_mod_kira::sound::static_sounds::KiraStaticSoundData"
//     }
//     fn short_type_path() -> &'static str {
//         "KiraStaticSoundData"
//     }
// }

pub type KiraStaticSoundAsset = SoundAsset<KiraStaticSoundData>;

pub struct StaticSoundFileLoader;

#[derive(Component)]
pub struct KiraStaticSoundHandle(pub Handle<KiraStaticSoundAsset>);

// This method for loading the sound was adapted from the bevy_kira_audio crate:
// See: https://github.com/NiklasEi/bevy_kira_audio/blob/main/src/source/ogg_loader.rs
impl AssetLoader for StaticSoundFileLoader {
    type Asset = KiraStaticSoundAsset;
    type Settings = ();
    type Error = KiraError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, KiraError> {
        let mut sound_bytes = vec![];
        reader.read_to_end(&mut sound_bytes).await?;
        debug!("Loading sound with {} bytes", sound_bytes.len());
        let sound = StaticSoundData::from_cursor(Cursor::new(sound_bytes))?;
        let asset: KiraStaticSoundAsset = KiraStaticSoundAsset {
            sound: KiraStaticSoundData(sound.clone()),
        };
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &[
            #[cfg(feature = "ogg")]
            "ogg",
            "oga",
            "spx",
            #[cfg(feature = "flac")]
            "flac",
            #[cfg(feature = "mp3")]
            "mp3",
            #[cfg(feature = "wav")]
            "wav",
        ]
    }
}
