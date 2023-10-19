use anyhow::Result;
use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::{debug, Component, Handle};
use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::BoxedFuture;
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::sound::SoundData;
use std::io::Cursor;

#[derive(TypeUuid, TypePath, Clone)]
#[uuid = "4e6dfb5e-8196-4974-8790-5bae8c01ac2d"]
pub struct SoundAsset<T>
where
    T: SoundData + Clone,
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
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let mut sound_bytes = vec![];
            for byte in bytes {
                sound_bytes.push(*byte);
            }
            debug!("Loading sound with {} bytes", sound_bytes.len());
            let sound = StaticSoundData::from_cursor(
                Cursor::new(sound_bytes),
                StaticSoundSettings::default(),
            )?;
            load_context.set_default_asset(LoadedAsset::new(KiraStaticSoundAsset {
                sound: KiraStaticSoundData(sound),
            }));
            Ok(())
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
