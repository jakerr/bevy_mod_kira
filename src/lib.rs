use std::fmt::Debug;
use std::fmt::Formatter;
use std::time::Duration;

use bevy::prelude::debug;

use bevy::prelude::EventReader;
use bevy::prelude::EventWriter;
use bevy::reflect::Reflect;
use bevy::{
    app::Plugin,
    prelude::{
        warn, AddAsset, AssetServer, Assets, Commands, Component, Entity, Handle, Local, Query,
        Res, ResMut, Resource,
    },
    time::{Time, Timer},
    utils::synccell::SyncCell,
};
use kira::sound::static_sound::PlaybackState;
use kira::sound::SoundData;
use kira::{
    manager::{
        backend::cpal::CpalBackend, error::PlaySoundError, AudioManager, AudioManagerSettings,
    },
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    tween::Tween,
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
#[derive(Component, Default, Reflect)]
#[reflect(Debug)]
pub struct KiraActiveSounds(#[reflect(ignore)] Vec<StaticSoundHandle>);

pub struct PlaySoundEvent<D: SoundData = StaticSoundData> {
    entity: Entity,
    sound: D,
}

impl Debug for KiraActiveSounds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KiraActiveSounds")
            .field("len", &self.0.len())
            .finish()
    }
}

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
    ) -> Result<StaticSoundHandle, PlaySoundError<()>> {
        if let Some(manager) = &mut self.manager {
            let exclusive_manager = manager.get();
            exclusive_manager.play(sound)
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
            .add_event::<PlaySoundEvent<StaticSoundData>>()
            .add_asset::<StaticSoundAsset>()
            .add_asset_loader(StaticSoundFileLoader)
            .add_startup_system(setup_sys)
            .add_system(trigger_play_sys)
            .add_system(do_play_sys)
            .add_system(cleanup_inactive_sounds_sys)
            // .add_system(tweak_mod_sys)
            .add_system(debug_kira_sys);
        app.register_type::<KiraActiveSounds>();
    }
}

fn setup_sys(mut commands: Commands, loader: Res<AssetServer>, mut time: ResMut<Time>) {
    time.set_wrap_period(Duration::from_secs(10));
    let a = loader.load("sfx.ogg");
    commands.spawn(KiraSoundHandle(a));
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

fn trigger_play_sys(
    assets: Res<Assets<StaticSoundAsset>>,
    mut query: Query<(Entity, &KiraSoundHandle)>,
    time: Res<Time>,
    mut looper: Local<TimerMs<1000>>,
    mut ev_play: EventWriter<PlaySoundEvent>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    for (eid, sound_handle) in query.iter_mut() {
        if assets.get(&sound_handle.0).is_none() {
            continue;
        }
        if let Some(sound_asset) = assets.get(&sound_handle.0) {
            ev_play.send(PlaySoundEvent {
                entity: eid,
                sound: sound_asset.sound.clone(),
            });
        }
    }
}

fn do_play_sys(
    mut commands: Commands,
    mut kira: ResMut<KiraContext>,
    mut query: Query<(Entity, Option<&mut KiraActiveSounds>)>,
    mut ev_play: EventReader<PlaySoundEvent>,
) {
    for event in ev_play.iter() {
        let s1 = kira.play(event.sound.clone()).unwrap();
        if let Ok((eid, active_sounds)) = query.get_mut(event.entity) {
            match active_sounds {
                Some(mut sounds) => {
                    sounds.0.push(s1);
                }
                None => {
                    let mut new_sounds = KiraActiveSounds::default();
                    new_sounds.0.push(s1);
                    commands.entity(eid).insert(new_sounds);
                }
            };
        }
    }
}

fn cleanup_inactive_sounds_sys(mut query: Query<&mut KiraActiveSounds>) {
    for mut sounds in query.iter_mut() {
        // first check for at least one stopped sound before deref mut to avoid spurious change
        // notifications notification. This is not yet profiled so may be a premature optimization.
        // note that `any` is short-circuiting so we don't need to worry about the cost iterating
        // over every sound.
        let needs_cleanup = sounds
            .0
            .iter()
            .any(|sound| sound.state() == PlaybackState::Stopped);

        if needs_cleanup {
            sounds
                .0
                .retain(|sound| sound.state() != PlaybackState::Stopped);
        }
    }
}

fn tweak_mod_sys(
    mut query: Query<&mut KiraActiveSounds>,
    time: Res<Time>,
    mut mod_wheel: Local<TimerMs<300>>,
) {
    mod_wheel.timer.tick(time.delta());
    if !mod_wheel.timer.just_finished() {
        return;
    }
    for mut active_sound_vec in query.iter_mut() {
        for active_sound in &mut active_sound_vec.0 {
            let _ = active_sound.set_panning(
                (time.elapsed_seconds_wrapped_f64() * 3.0).sin(),
                Tween::default(),
            );
        }
    }
}

struct DebugKiraManager<'a> {
    manager: &'a AudioManager,
}

struct DebugKiraContext<'a> {
    manager: Option<DebugKiraManager<'a>>,
}

impl<'a> Debug for DebugKiraContext<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KiraContext")
            .field("manager", &self.manager)
            .finish()
    }
}

impl<'a> Debug for DebugKiraManager<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Manager")
            .field("state", &self.manager.state())
            .field("num_sounds", &self.manager.num_sounds())
            .field("num_sub_tracks", &self.manager.num_sub_tracks())
            .field("num_clocks", &self.manager.num_clocks())
            .field("sound_capacity", &self.manager.sound_capacity())
            .field("sub_track_capacity", &self.manager.sub_track_capacity())
            .field("clock_capacity", &self.manager.clock_capacity())
            .finish()
    }
}

// Because KiraContext uses a sync cell we have to jump through some hoops to make a debug type that
// can be printed via the debug trait which takes a non-mutable reference.
impl<'a> From<&'a mut KiraContext> for DebugKiraContext<'a> {
    fn from(context: &'a mut KiraContext) -> Self {
        let manager_borrow = context.get_manager();
        Self {
            manager: match manager_borrow {
                Some(manager) => Some(DebugKiraManager { manager }),
                None => None,
            },
        }
    }
}

fn debug_kira_sys(
    mut kira: ResMut<KiraContext>,
    active: Query<(Entity, &KiraActiveSounds)>,
    time: Res<Time>,
    mut looper: Local<TimerMs<1000>>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    let context: DebugKiraContext = kira.as_mut().into();
    for (eid, active) in active.iter() {
        debug!("Eid: {:?} has {} active sounds.", eid, active.0.len());
    }
    debug!("Context: {:?}", context);
}
