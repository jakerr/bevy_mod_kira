use std::{
    f64::consts::PI,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use bevy::prelude::*;
use bevy_mod_kira::{DynamicSoundHandle, KiraPlaySoundEvent, KiraPlayingSounds, KiraPlugin};
use kira::{
    dsp::Frame,
    sound::{Sound, SoundData},
};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(KiraPlugin)
        .add_startup_system(setup_sys)
        .add_system(trigger_play_sys)
        .add_system(handles_sys)
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

#[derive(Component, Clone)]
pub struct MySoundData;
struct MySound {
    tone: f64,
    phase: f64,
    len: f64,
    stopped: Arc<AtomicBool>,
}
#[derive(Debug)]
pub struct MySoundHandle {
    stopped: Arc<AtomicBool>,
}

impl DynamicSoundHandle for MySoundHandle {
    fn state(&self) -> kira::sound::static_sound::PlaybackState {
        self.stopped
            .load(std::sync::atomic::Ordering::Relaxed)
            .then(|| kira::sound::static_sound::PlaybackState::Stopped)
            .unwrap_or(kira::sound::static_sound::PlaybackState::Playing)
    }
}

impl Sound for MySound {
    fn track(&mut self) -> kira::track::TrackId {
        kira::track::TrackId::Main
    }

    fn process(
        &mut self,
        dt: f64,
        _clock_info_provider: &kira::clock::clock_info::ClockInfoProvider,
    ) -> kira::dsp::Frame {
        self.phase += dt;
        let tone = (self.phase * self.tone * 2.0 * PI).sin() as f32;
        let progress = self.phase / self.len;
        let max = 0.5;
        let scaled = max * tone * (progress * PI).sin() as f32;
        if self.phase > self.len {
            self.stopped
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Frame {
            left: scaled,
            right: scaled,
        }
    }

    fn finished(&self) -> bool {
        self.stopped.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl SoundData for MySoundData {
    type Handle = MySoundHandle;
    type Error = ();

    fn into_sound(self) -> Result<(Box<dyn kira::sound::Sound>, Self::Handle), Self::Error> {
        let stopped = Arc::new(AtomicBool::new(false));
        Ok((
            Box::new(MySound {
                // Middle c
                tone: 261.63,
                phase: 0.0,
                len: 3.0,
                stopped: stopped.clone(),
            }),
            MySoundHandle { stopped },
        ))
    }
}

fn setup_sys(mut commands: Commands) {
    commands.spawn(MySoundData);
}

fn trigger_play_sys(
    my_sound: Query<(Entity, &MySoundData)>,
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
    for (eid, my_sound_data) in my_sound.iter() {
        ev_play.send(KiraPlaySoundEvent::new(eid, my_sound_data.clone()));
    }
}

fn handles_sys(sounds: Query<&KiraPlayingSounds>) {
    for active_sounds in sounds.iter() {
        let mut count = 0usize;
        for handle in active_sounds.dynamic_handels::<MySoundHandle>() {
            count += 1;
            info!("MySound..., {:?}, {}", handle, count);
        }
    }
}
