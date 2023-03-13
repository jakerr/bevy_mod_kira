use bevy::{
    app::Plugin,
    prelude::{
        warn, AddAsset, AssetServer, Assets, Commands, Component, Entity, Handle, Query, Res,
        ResMut, Resource,
    },
    reflect::TypeUuid,
    utils::synccell::SyncCell,
};
use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::SoundData,
};
use static_sound_loader::{SoundAsset, StaticSoundAsset, StaticSoundFileLoader};

mod err;
mod static_sound_loader;

#[derive(Resource)]
pub struct KiraContext {
    manager: Option<SyncCell<AudioManager>>,
}

impl Default for KiraContext {
    fn default() -> Self {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default());
        if let Err(ref error) = manager {
            warn!("Error creating KiraContext: {}", error);
        }
        Self {
            manager: manager.ok().map(|m| SyncCell::new(m)),
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

    pub fn get_manager(&mut self) -> Option<&mut AudioManager> {
        if let Some(manager) = &mut self.manager {
            let exclusive_manager = manager.get();
            return Some(exclusive_manager);
        }
        None
    }

    pub fn play_asset<T: SoundData + Clone + Send + Sync + 'static>(
        &mut self,
        assets: &Assets<SoundAsset<T>>,
        handle: &Handle<SoundAsset<T>>,
    ) where
        SoundAsset<T>: TypeUuid,
    {
        if let Some(sound_asset) = assets.get(&handle) {
            let manager = self.get_manager().unwrap();
            let _ = manager.play(sound_asset.sound.clone());
        }
    }
}

pub struct KiraPlugin;

impl Plugin for KiraPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<KiraContext>()
            .add_asset::<StaticSoundAsset>()
            .add_asset_loader(StaticSoundFileLoader)
            .add_startup_system(setup_sys)
            .add_system(play_sys);
    }
}

#[derive(Component)]
struct ToPlay(Handle<StaticSoundAsset>);

fn setup_sys(mut commands: Commands, loader: Res<AssetServer>) {
    let a = loader.load("sfx.ogg");
    commands.spawn(ToPlay(a));
}

fn play_sys(
    mut commands: Commands,
    mut kira: ResMut<KiraContext>,
    assets: Res<Assets<StaticSoundAsset>>,
    query: Query<(Entity, &ToPlay)>,
) {
    for (eid, to_play) in query.iter() {
        if assets.get(&to_play.0).is_none() {
            continue;
        }
        kira.play_asset(&assets, &to_play.0);
        commands.entity(eid).despawn();
    }
}
