use anyhow::Result;
use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::info;
use bevy::reflect::impl_type_uuid;
use bevy::utils::BoxedFuture;
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::sound::SoundData;
use std::io::Cursor;

#[derive(Clone)]
pub struct SoundAsset<T>
where
    T: SoundData + Clone,
{
    pub sound: T,
}

pub type KiraStaticSoundAsset = SoundAsset<StaticSoundData>;
impl_type_uuid!(KiraStaticSoundAsset, "4e6dfb5e-8196-4974-8790-5bae8c01ac2d");

pub struct StaticSoundFileLoader;

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
            info!("Loading sound with {} bytes", sound_bytes.len());
            let sound = StaticSoundData::from_cursor(
                Cursor::new(sound_bytes),
                StaticSoundSettings::default(),
            )?;
            load_context.set_default_asset(LoadedAsset::new(KiraStaticSoundAsset { sound }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ogg"]
    }
}
