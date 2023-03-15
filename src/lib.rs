use bevy::{
    app::Plugin,
    prelude::{
        warn, AddAsset, AssetServer, Assets, Commands, Component, Entity, Handle, Local, Query,
        Res, ResMut, Resource,
    },
    time::{Time, Timer},
    utils::synccell::SyncCell,
};
use kira::{
    manager::{
        backend::cpal::CpalBackend, error::PlaySoundError, AudioManager, AudioManagerSettings,
    },
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
use static_sound_loader::{StaticSoundAsset, StaticSoundFileLoader};

mod err;
mod static_sound_loader;

#[derive(Resource)]
pub struct KiraContext {
    manager: Option<SyncCell<AudioManager>>,
}

#[derive(Component)]
pub struct KiraSoundHandle(Handle<StaticSoundAsset>);
#[derive(Component)]
pub struct KiraSoundController(StaticSoundHandle);

impl Default for KiraContext {
    fn default() -> Self {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default());
        if let Err(ref error) = manager {
            warn!("Error creating KiraContext: {}", error);
        }
        Self {
            manager: manager.ok().map(SyncCell::new),
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

    // Takes the same params as AudioManager::play calls the internal manager and then converts the handle into a bevy component type.
    pub fn play(
        &mut self,
        sound: StaticSoundData,
    ) -> Result<KiraSoundController, PlaySoundError<()>> {
        if let Some(manager) = &mut self.manager {
            let exclusive_manager = manager.get();
            exclusive_manager.play(sound).map(KiraSoundController)
        } else {
            Err(PlaySoundError::IntoSoundError(()))
        }
    }

    pub fn get_manager(&mut self) -> Option<&mut AudioManager> {
        if let Some(manager) = &mut self.manager {
            let exclusive_manager = manager.get();
            return Some(exclusive_manager);
        }
        None
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

fn setup_sys(mut commands: Commands, loader: Res<AssetServer>) {
    let a = loader.load("sfx.ogg");
    commands.spawn(KiraSoundHandle(a));
}

struct Looper {
    timer: Timer,
}

impl Default for Looper {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(10.0, bevy::time::TimerMode::Repeating),
        }
    }
}

fn play_sys(
    mut commands: Commands,
    mut kira: ResMut<KiraContext>,
    assets: Res<Assets<StaticSoundAsset>>,
    query: Query<(Entity, &KiraSoundHandle)>,
    time: Res<Time>,
    mut looper: Local<Looper>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    for (eid, sound_handle) in query.iter() {
        if assets.get(&sound_handle.0).is_none() {
            continue;
        }
        if let Some(sound_asset) = assets.get(&sound_handle.0) {
            let s = kira.play(sound_asset.sound.clone()).unwrap();
            commands.entity(eid).remove::<KiraSoundController>();
            commands.entity(eid).insert(s);
        }
    }
}
