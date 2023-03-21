use std::time::Duration;

use bevy::prelude::*;
use bevy_mod_kira::{KiraPlaySoundEvent, KiraPlugin, KiraSoundHandle, KiraStaticSoundAsset};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(KiraPlugin)
        .add_startup_system(setup_sys)
        .add_system(trigger_play_sys)
        .run();
}

struct TimerMs<const N: i32> {
    timer: Timer,
}

impl<const N: i32> Default for TimerMs<N> {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(N as f32 / 1000.0, TimerMode::Repeating);
        // We'd like our Local timer to trigger soon after creation for the first iteration.
        timer.tick(Duration::from_millis(N as u64 - 500));
        Self { timer }
    }
}

fn setup_sys(mut commands: Commands, loader: Res<AssetServer>) {
    let a = loader.load("sfx.ogg");
    // Creates an entity with a KiraSoundHandle component the sound handle will eventually resolve
    // to the KiraStaticSoundAsset once the asset has loaded.
    commands.spawn(KiraSoundHandle(a));
}

fn trigger_play_sys(
    assets: Res<Assets<KiraStaticSoundAsset>>,
    query: Query<(Entity, &KiraSoundHandle)>,
    time: Res<Time>,
    // This timer is used to trigger the sound playback every 5 seconds.
    mut looper: Local<TimerMs<5000>>,
    // This event writer is our interface to start sounds with the KiraPlugin.
    mut ev_play: EventWriter<KiraPlaySoundEvent>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    for (eid, sound_handle) in query.iter() {
        if let Some(sound_asset) = assets.get(&sound_handle.0) {
            // The KiraPlaySoundEvent takes a entity id and a sound data object. When the sound
            // begins playing a KiraActiveSounds component will be added (or extended if it already
            // exists) to the entity for the given id  to contain the sound handle while the sound
            // plays.
            //
            // KiraActiveSounds can later be queried from another system to interact with playing
            // sounds and perform any number of actions provided by the Kira StaticSoundHandle api.
            let sound_data = sound_asset.sound.clone();
            ev_play.send(KiraPlaySoundEvent::new(eid, sound_data));
        }
    }
}
