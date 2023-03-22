use std::time::Duration;

use bevy::prelude::*;
use bevy_egui::{
    egui::{
        self,
        plot::{BarChart, HLine},
    },
    EguiContexts, EguiPlugin,
};
use bevy_mod_kira::{
    KiraAddTrackEvent, KiraPlaySoundEvent, KiraPlugin, KiraSoundHandle, KiraStaticSoundAsset,
    KiraTracks,
};
use kira::track::{effect::reverb::ReverbHandle, TrackBuilder};

mod effects;
use effects::{LevelMonitorBuilder, LevelMonitorHandle};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy mod Kira - Spectral Analysis".into(),
                resolution: (800., 460.).into(),
                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(KiraPlugin)
        .add_plugin(EguiPlugin)
        .add_startup_system(setup_sys)
        .add_system(trigger_play_sys)
        .add_system(ui_sys)
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

#[derive(Component)]
struct LevelsHandle(LevelMonitorHandle);

fn setup_sys(
    mut commands: Commands,
    loader: Res<AssetServer>,
    mut ev_tracks: ResMut<Events<KiraAddTrackEvent>>,
) {
    let a = loader.load("sfx.ogg");
    // Creates an entity with a KiraSoundHandle component the sound handle will eventually resolve
    // to the KiraStaticSoundAsset once the asset has loaded.
    let mut entity = commands.spawn(KiraSoundHandle(a));

    // Next we add a track to the channel and adding a reverb effect to the track. Both of these
    // steps are optional. If you don't specify a track when playing a sound it will play on
    // a default Main track.
    let monitor = LevelMonitorBuilder::new(1.0);
    let mut track = TrackBuilder::new();

    // The reverb handle is returned directly from the track builder even before we've sent it
    // to Kira so it's our responsibility to hold onto it in a component if we want to be able
    // to modify it later.
    let monitor_handle = track.add_effect(monitor);
    entity.insert(LevelsHandle(monitor_handle));

    // We send the track builder to Kira along with the entity id for this channel. Once added
    // the KiraPlugin will add the track to KiraTracks component on the channel entity.
    ev_tracks.send(KiraAddTrackEvent::new(entity.id(), track));
}

fn trigger_play_sys(
    assets: Res<Assets<KiraStaticSoundAsset>>,
    query: Query<(Entity, &KiraSoundHandle, &KiraTracks)>,
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
    for (eid, sound_handle, tracks) in query.iter() {
        if let Some(sound_asset) = assets.get(&sound_handle.0) {
            // The KiraPlaySoundEvent takes a entity id and a sound data object. When the sound
            // begins playing a KiraActiveSounds component will be added (or extended if it already
            // exists) to the entity for the given id  to contain the sound handle while the sound
            // plays.
            //
            // KiraActiveSounds can later be queried from another system to interact with playing
            // sounds and perform any number of actions provided by the Kira StaticSoundHandle api.
            let track = &tracks.0[0];
            let sound_data = sound_asset.sound.clone();
            let sound = sound_data.with_modified_settings(|settings| settings.track(track.id()));
            ev_play.send(KiraPlaySoundEvent::new(eid, sound));
        }
    }
}

#[derive(Default)]
struct Peaks(f64, f64);

fn ui_sys(mut ctx: EguiContexts, mut query: Query<(&mut LevelsHandle)>, mut peaks: Local<Peaks>) {
    let mut left = 0.;
    let mut right = 0.;
    let mut monitor_handle = query.single_mut();
    if let Ok(levels) = monitor_handle.0.get_sample() {
        left = levels.left;
        right = levels.right;
        peaks.0 = if peaks.0 > levels.left_peak {
            peaks.0 * 0.99
        } else {
            levels.left_peak
        };
        peaks.1 = if peaks.1 > levels.left_peak {
            peaks.1 * 0.99
        } else {
            levels.left_peak
        };
    }
    egui::Window::new("Spectral Analysis")
        .default_width(800.0)
        .default_height(460.0)
        .show(ctx.ctx_mut(), |ui| {
            // Plot two bars one for left and one for right. Make the heights relative to the left and right levels.
            let left_bar = egui::plot::Bar::new(-0.5, left);
            let right_bar = egui::plot::Bar::new(0.5, right);
            let plot = egui::plot::Plot::new("Bar Chart")
                .legend(egui::plot::Legend::default())
                .include_x(-1.0)
                .include_x(1.0)
                .include_y(0.0)
                .include_y(1.0);
            let max_peak = peaks.0.max(peaks.1);
            plot.show(ui, |plot_ui| {
                plot_ui.hline(HLine::new(max_peak));
                plot_ui.bar_chart(BarChart::new(vec![left_bar, right_bar]));
            });
        });
}
