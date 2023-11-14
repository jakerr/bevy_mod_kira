use anyhow::Result;
use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::{debug, Component, Handle};
use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::BoxedFuture;
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

#[derive(TypeUuid, TypePath, Clone, Asset)]
#[uuid = "4e6dfb5e-8196-4974-8790-5bae8c01ac2d"]
pub struct SoundAsset<T>
where
    T: TypePath + Send + Sync + SoundData + Clone,
{
    pub sound: T,
}

#[derive(Clone, TypeUuid, TypePath)]
#[uuid = "10eed7c5-cfaa-49c7-9fa4-c17735e5ef25"]
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

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, KiraError>> {
        Box::pin(async move {
            let mut sound_bytes = vec![];
            reader.read_to_end(&mut sound_bytes).await?;
            debug!("Loading sound with {} bytes", sound_bytes.len());
            let sound = StaticSoundData::from_cursor(
                Cursor::new(sound_bytes),
                StaticSoundSettings::default(),
            )?;
            let asset: KiraStaticSoundAsset = KiraStaticSoundAsset {
                sound: KiraStaticSoundData(sound.clone()),
            };
            Ok(asset)
        })
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
