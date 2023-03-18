use std::time::Duration;

use bevy::{
    prelude::{
        debug, error, App, AssetServer, Assets, Commands, Component, Entity, EventWriter,
        Local, Query, Res, ResMut,
    },
    reflect::Reflect,
    time::{Time, Timer},
    DefaultPlugins,
};
use bevy_mod_kira::{
    AddTrackEvent, KiraAssociatedTracks, KiraPlugin, KiraSoundHandle, PlaySoundEvent,
    StaticSoundAsset,
};
use kira::{
    track::{effect::reverb::ReverbHandle, TrackBuilder},
    tween::Tween,
};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(KiraPlugin)
        .add_startup_system(setup_sys)
        .add_system(setup_track_sys)
        .add_system(trigger_play_sys)
        .add_system(tweak_reverb_sys)
        .register_type::<TrackOneReverb>()
        .run();
}

struct TimerMs<const N: i32> {
    timer: Timer,
}

#[derive(Component, Reflect)]
struct TrackOneReverb(#[reflect(ignore)] ReverbHandle);

impl<const N: i32> Default for TimerMs<N> {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(N as f32 / 1000.0, bevy::time::TimerMode::Repeating),
        }
    }
}

fn setup_sys(mut commands: Commands, loader: Res<AssetServer>, mut time: ResMut<Time>) {
    time.set_wrap_period(Duration::from_secs(60));
    let a = loader.load("sfx.ogg");
    commands.spawn(KiraSoundHandle(a));
}

fn setup_track_sys(
    mut commands: Commands,
    mut query: Query<(Entity, &KiraSoundHandle)>,
    mut ev_tracks: EventWriter<AddTrackEvent>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    debug!(
        "Setting up track. Sound handle counts {}",
        query.iter().count()
    );
    for (eid, _sound_handle) in query.iter_mut() {
        debug!("Setting up track. for sound handle.");
        let reverb = kira::track::effect::reverb::ReverbBuilder::new()
            .mix(1.0)
            .stereo_width(1.0);
        let mut track = TrackBuilder::new();
        let reverb_handle = track.add_effect(reverb);
        commands.entity(eid).insert(TrackOneReverb(reverb_handle));
        ev_tracks.send(AddTrackEvent::new(eid, track));
        *done = true;
    }
}

fn tweak_reverb_sys(
    mut query: Query<&mut TrackOneReverb>,
    time: Res<Time>,
    mut mod_wheel: Local<TimerMs<5000>>,
) {
    mod_wheel.timer.tick(time.delta());
    if !mod_wheel.timer.just_finished() {
        return;
    }
    for mut reverb in query.iter_mut() {
        let mut tween = Tween::default();
        tween.duration = Duration::from_millis(1000);
        let value = (time.elapsed_seconds_wrapped_f64() * 3.0).sin() / 2.0 + 0.5;
        debug!("Tweaking reverb over 1 second. Target value: {}", value);
        let Ok(_) = reverb.0.set_mix(
           value, 
           tween,
        ) else { error!("Error while tweaking reverb."); continue; };
    }
}

fn trigger_play_sys(
    assets: Res<Assets<StaticSoundAsset>>,
    mut query: Query<(Entity, &KiraSoundHandle, &KiraAssociatedTracks)>,
    time: Res<Time>,
    mut looper: Local<TimerMs<5000>>,
    mut ev_play: EventWriter<PlaySoundEvent>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    for (eid, sound_handle, tracks) in query.iter_mut() {
        if assets.get(&sound_handle.0).is_none() {
            continue;
        }
        if let Some(sound_asset) = assets.get(&sound_handle.0) {
            let sound = sound_asset.sound.with_modified_settings(|mut settings| {
                if let Some(track1) = tracks.0.first() {
                    settings = settings.track(track1);
                } else {
                    error!("No track found for sound handle.");
                }
                settings
            });
            ev_play.send(PlaySoundEvent::new(eid, sound));
        }
    }
}
