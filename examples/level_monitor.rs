use std::time::Duration;

use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiPlugin,
    egui::{self},
};
use bevy_mod_kira::{
    KiraContext, KiraPlaySoundEvent, KiraPlugin, KiraStaticSoundAsset, KiraStaticSoundHandle,
};
use egui_plot::{BarChart, HLine, LineStyle};
use kira::track::{TrackBuilder, TrackHandle};

mod effects;
use effects::{LevelMonitorBuilder, LevelMonitorHandle};
const SAMPLES: usize = 16;
mod color_utils;
use color_utils::*;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy mod Kira - Level Monitor".into(),
                resolution: (800., 460.).into(),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((KiraPlugin, EguiPlugin))
        .add_systems(Startup, setup_sys)
        .add_systems(Update, (trigger_play_sys, ui_sys))
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
struct LevelsHandle<const N: usize>(LevelMonitorHandle<N>);

#[derive(Component)]
struct Panning(f64);

#[derive(Component)]
struct Track(TrackHandle);

fn setup_sys(mut commands: Commands, loader: Res<AssetServer>, mut kira: NonSendMut<KiraContext>) {
    // See the play_sound.rs example for more detailed comments on how to load and play sounds.
    let a = loader.load("hit.ogg");
    let mut entity = commands.spawn(KiraStaticSoundHandle(a));

    // This LevelMonitorBuilder is defined in the examples directory. We're defined this custom
    // effect type to extract samples from the track's stream so that we can show a level meter.
    let monitor = LevelMonitorBuilder::<SAMPLES>;

    let mut track = TrackBuilder::new();
    let monitor_handle = track.add_effect(monitor);

    // Hold onto the effect handle so that we can exract the samples from it in another system.
    entity.insert(LevelsHandle(monitor_handle));
    entity.insert(Panning(0.5));
    let track_handle = kira
        .add_track(track)
        .expect("Failed to add track to KiraContext");
    entity.insert(Track(track_handle));
}

fn trigger_play_sys(
    assets: Res<Assets<KiraStaticSoundAsset>>,
    query: Query<(Entity, &KiraStaticSoundHandle, &Track, &Panning)>,
    time: Res<Time>,
    mut looper: Local<TimerMs<1000>>,
    mut ev_play: EventWriter<KiraPlaySoundEvent>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    for (eid, sound_handle, track, panning) in query.iter() {
        if let Some(sound_asset) = assets.get(&sound_handle.0) {
            let sound_data = sound_asset.sound.clone();
            let sound = sound_data.0.with_modified_settings(|settings| {
                settings.output_destination(track.0.id()).panning(panning.0)
            });
            ev_play.send(KiraPlaySoundEvent::new(eid, sound));
        }
    }
}

#[derive(Default)]
struct Peaks {
    left: f32,
    right: f32,
    left_peak: f32,
    right_peak: f32,
}

fn dbs_from_rms(rms: f32) -> f32 {
    100.0 + (20.0 * rms.log10()).max(-100.0)
}

fn ui_sys(
    mut ctx: EguiContexts,
    mut query: Query<(&mut LevelsHandle<SAMPLES>, &mut Panning)>,
    mut peaks: Local<Peaks>,
) {
    let (mut monitor_handle, mut panning) = query.single_mut();
    // Pull a sample containing a window of SAMPLES frames from the LevelMonitor effect.
    // See the level_monitor/mod.rs file to see how these samples are extracted.
    if let Ok(levels) = monitor_handle.0.get_sample() {
        let samples = levels.window.len() as f32;
        // Do some math to determine the decible level of the left and right channels.
        let squares = levels
            .window
            .iter()
            .map(|x| (x.left * x.left, x.right * x.right))
            .fold((0.0, 0.0), |(l, r), (nl, nr)| (l + nl, r + nr));
        let rms = ((squares.0 / samples).sqrt(), (squares.1 / samples).sqrt());
        // 0 - 100 represents -100 to 0 dB
        let dbs = (dbs_from_rms(rms.0), dbs_from_rms(rms.1));

        let (left, right) = dbs;
        peaks.left = peaks.left.max(left);
        peaks.left_peak = peaks.left_peak.max(left);
        peaks.right = peaks.right.max(right);
        peaks.right_peak = peaks.right_peak.max(right);
    }

    // The rest is just egui code to draw the level meters.
    let fast_decay = 0.90;
    let slow_decay = 0.99;
    peaks.left *= fast_decay;
    peaks.right *= fast_decay;
    peaks.right_peak *= slow_decay;
    peaks.left_peak *= slow_decay;
    egui::Window::new("Level Monitor")
        .default_width(600.0)
        .default_height(400.0)
        .show(ctx.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Panning");
                ui.add(egui::Slider::new(&mut panning.0, 0.0..=1.0).text("Panning"));
            });
            // Plot two bars one for left and one for right. Make the heights relative to the left and right levels.
            let left_bar = egui_plot::Bar::new(-0.5, peaks.left as f64);
            let right_bar = egui_plot::Bar::new(0.5, peaks.right as f64);
            let plot = egui_plot::Plot::new("Levels")
                .legend(egui_plot::Legend::default())
                .label_formatter(|_name, value| format!("{:.2} db", value.y - 100.0))
                .show_background(false)
                .include_x(-1.0)
                .include_x(1.0)
                .include_y(100.0)
                .include_y(0.0);
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0.0,
                dark_color(Pallete::DeepBlue),
            );
            let left_color = Pallete::MintGreen;
            let right_color = Pallete::AquaBlue;
            plot.show(ui, |plot_ui| {
                plot_ui.hline(
                    HLine::new(peaks.left_peak)
                        .style(LineStyle::Dashed { length: 5.0 })
                        .width(2.0)
                        .name("Left")
                        .color(left_color),
                );
                plot_ui.hline(
                    HLine::new(peaks.right_peak)
                        .style(LineStyle::Dashed { length: 5.0 })
                        .width(2.0)
                        .name("Right")
                        .color(right_color),
                );
                plot_ui.bar_chart(BarChart::new(vec![left_bar]).color(left_color));
                plot_ui.bar_chart(BarChart::new(vec![right_bar]).color(right_color));
            });
        });
}
