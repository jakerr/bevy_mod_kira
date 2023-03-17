use bevy::{
    prelude::{
        debug, error, warn, App, AssetServer, Assets, BuildChildren, Commands, Component, Entity,
        EventWriter, Local, Parent, Query, Res,
    },
    reflect::Reflect,
    time::{Time, Timer},
    utils::HashMap,
    DefaultPlugins,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mod_kira::{
    AddClockEvent, AddTrackEvent, KiraAssociatedClocks, KiraAssociatedTracks, KiraPlugin,
    KiraSoundHandle, PlaySoundEvent, StaticSoundAsset,
};
use kira::track::{effect::reverb::ReverbHandle, TrackBuilder};

const BPM: f64 = 110.0;
const BPS: f64 = BPM / 60.0;
const STEP_PER_BEAT: usize = 4;
const STEPS: usize = STEP_PER_BEAT * 4;
const STEP_PER_SEC: f64 = BPS * STEP_PER_BEAT as f64;

// We'll trigger a system to queue next sounds at this rate (in ms). We trigger at some division of
// the clock tick rate so that we are sure that we are enqueueing the next step in time.
const PLAYHEAD_RESOLUTION_MS: u32 = ((1000.0 / STEP_PER_SEC) * 0.8) as u32;

#[derive(Component, Reflect)]
struct DrumPattern {
    steps: [bool; STEPS],
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(KiraPlugin)
        .add_plugin(WorldInspectorPlugin::new())
        .add_startup_system(setup_sys)
        .add_system(playback_sys)
        .register_type::<TrackOneReverb>()
        .run();
}

struct TimerMs<const N: u32> {
    timer: Timer,
}

#[derive(Component, Reflect)]
struct TrackOneReverb(#[reflect(ignore)] ReverbHandle);

impl<const N: u32> Default for TimerMs<N> {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(N as f32 / 1000.0, bevy::time::TimerMode::Repeating),
        }
    }
}

fn setup_sys(
    mut commands: Commands,
    loader: Res<AssetServer>,
    mut ev_tracks: EventWriter<AddTrackEvent>,
    mut ev_clocks: EventWriter<AddClockEvent>,
) {
    debug!("BPM: {}", BPM);
    debug!("BPS: {}", BPS);
    debug!("STEP_PER_BEAT: {}", STEP_PER_BEAT);
    debug!("STEPS: {}", STEPS);
    debug!("STEP_PER_SEC: {}", STEP_PER_SEC);
    debug!("PLAY_HEAD_RESOLUTION_MS: {}", PLAYHEAD_RESOLUTION_MS);
    debug!("Setting up track.");
    let a = loader.load("kick.ogg");
    let mut track_entity = commands.spawn(KiraSoundHandle(a));
    let reverb = kira::track::effect::reverb::ReverbBuilder::new()
        .mix(0.0)
        .stereo_width(0.0);
    let mut track = TrackBuilder::new();
    let reverb_handle = track.add_effect(reverb);
    track_entity.insert(TrackOneReverb(reverb_handle));
    ev_tracks.send(AddTrackEvent::new(track_entity.id(), track));
    ev_clocks.send(AddClockEvent::new(
        track_entity.id(),
        kira::ClockSpeed::TicksPerSecond(STEP_PER_SEC),
    ));
    track_entity.with_children(|parent| {
        let x = true;
        let o = false;
        #[rustfmt::skip]
        parent.spawn(DrumPattern {
            steps: [
                x, o, o, o,
                x, o, o, o,
                x, o, o, x,
                x, o, x, o,
            ],
        });
    });
}

#[derive(Default)]
struct LastTicks(HashMap<Entity, u64>);

fn playback_sys(
    assets: Res<Assets<StaticSoundAsset>>,
    tracks: Query<(
        &KiraSoundHandle,
        &KiraAssociatedTracks,
        &KiraAssociatedClocks,
    )>,
    patterns: Query<(Entity, &DrumPattern, &Parent)>,
    time: Res<Time>,
    mut looper: Local<TimerMs<PLAYHEAD_RESOLUTION_MS>>,
    mut ev_play: EventWriter<PlaySoundEvent>,
    mut last_ticks: Local<LastTicks>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    for (pattern_id, pattern, parent) in patterns.iter() {
        if let Ok((sound, tracks, clocks)) = tracks.get(parent.get()) {
            let clock = clocks.0.first().unwrap();
            let clock_ticks = clock.time().ticks;
            let last_tick = last_ticks.0.get(&pattern_id).copied().unwrap_or(u64::MAX);
            if clock_ticks == last_tick {
                continue;
            }
            if (clock_ticks - last_tick) > 1 {
                warn!("Missed a tick! cur: {} last: {}", clock_ticks, last_tick);
            }
            last_ticks.0.insert(pattern_id, clock_ticks);
            let next_play_step = clock_ticks as usize % STEPS;
            if pattern.steps[next_play_step] {
                let sound_asset = assets.get(&sound.0).unwrap();
                let sound = sound_asset.sound.with_modified_settings(|mut settings| {
                    if let Some(track1) = tracks.0.first() {
                        // We calculate next_play_step as the the step at current clock time, but we
                        // want to start the sound right at precise tick so every sound will be
                        // triggered at a 1 tick offset.
                        settings = settings.track(track1).start_time(clock.time() + 1)
                    } else {
                        error!("No track found for sound handle.");
                    }
                    settings
                });
                ev_play.send(PlaySoundEvent::new(pattern_id, sound));
            }
        }
    }
}
