use bevy::{
    prelude::{App, Assets, Entity, EventWriter, Local, Query, Res},
    time::{Time, Timer},
    DefaultPlugins,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mod_kira::{KiraPlugin, KiraSoundHandle, PlaySoundEvent, StaticSoundAsset};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(KiraPlugin)
        .add_plugin(WorldInspectorPlugin::new())
        .add_system(trigger_play_sys)
        .run();
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
    mut looper: Local<TimerMs<5000>>,
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
            ev_play.send(PlaySoundEvent::new(eid, sound_asset.sound.clone()));
        }
    }
}
