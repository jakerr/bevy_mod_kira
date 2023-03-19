#![feature(split_array)]

use bevy::{
    ecs::system::EntityCommands,
    prelude::{
        debug, error, warn, App, AssetServer, Assets, BuildChildren, Children, Commands, Component,
        Entity, EventWriter, Local, Parent, Query, Res, With,
    },
    reflect::Reflect,
    utils::HashMap,
    DefaultPlugins,
};
use bevy_egui::{
    egui::{self, epaint::Hsva, Pos2, Rgba, Stroke},
    EguiContexts, EguiPlugin,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mod_kira::{
    AddClockEvent, AddTrackEvent, KiraAssociatedClocks, KiraAssociatedTracks, KiraPlugin,
    KiraSoundHandle, PlaySoundEvent, StaticSoundAsset,
};
use egui::Color32;
use egui::Id;
use egui::Sense;
use egui_extras::{Size, StripBuilder};
use kira::{
    track::{effect::reverb::ReverbHandle, TrackBuilder},
    tween::Tween,
};

const BPM: f64 = 110.0;
const BPS: f64 = BPM / 60.0;
const STEP_PER_BEAT: usize = 4;
const STEPS: usize = STEP_PER_BEAT * 4;
const STEP_PER_SEC: f64 = BPS * STEP_PER_BEAT as f64;

// Non const of the same as above for use in the UI.
fn steps_per_sec(bpm: f64) -> f64 {
    bpm / 60.0 * STEP_PER_BEAT as f64
}

// We'll trigger a system to queue next sounds at this rate (in ms). We trigger at some division of
// the clock tick rate so that we are sure that we are enqueueing the next step in time.
const PLAYHEAD_RESOLUTION_MS: u32 = ((1000.0 / STEP_PER_SEC) * 0.8) as u32;

#[derive(Component, Default, Reflect)]
struct DrumPattern {
    steps: [bool; STEPS],
}

#[derive(Component, Reflect)]
struct Bpm(f64);

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(KiraPlugin)
        .add_plugin(WorldInspectorPlugin::new())
        // .add_plugin(EguiPlugin)
        .add_startup_system(setup_sys)
        .add_system(playback_sys)
        .add_system(ui_sys)
        .register_type::<TrackOneReverb>()
        .run();
}

#[derive(Component, Reflect)]
struct TrackOneReverb(#[reflect(ignore)] ReverbHandle);

fn add_instrument_channel(
    asset: &str,
    parent: &mut EntityCommands,
    loader: &AssetServer,
    ev_tracks: &mut EventWriter<AddTrackEvent>,
) {
    let a = loader.load(asset);
    parent.with_children(|parent| {
        let mut channel = parent.spawn(KiraSoundHandle(a));
        let reverb = kira::track::effect::reverb::ReverbBuilder::new()
            .mix(0.0)
            .stereo_width(0.0);
        let mut track = TrackBuilder::new();
        let reverb_handle = track.add_effect(reverb);
        ev_tracks.send(AddTrackEvent::new(channel.id(), track));
        channel.insert(TrackOneReverb(reverb_handle));
        channel.with_children(|parent| {
            parent.spawn(DrumPattern::default());
        });
    });
}

fn setup_sys(
    mut commands: Commands,
    loader: Res<AssetServer>,
    mut ev_tracks: EventWriter<AddTrackEvent>,
    mut ev_clocks: EventWriter<AddClockEvent>,
) {
    let mut drum_machine = commands.spawn(Bpm(BPM));
    ev_clocks.send(AddClockEvent::new(
        drum_machine.id(),
        kira::ClockSpeed::TicksPerSecond(STEP_PER_SEC),
    ));
    add_instrument_channel("kick.ogg", &mut drum_machine, &loader, &mut ev_tracks);
    add_instrument_channel("snare.ogg", &mut drum_machine, &loader, &mut ev_tracks);
    add_instrument_channel("hat.ogg", &mut drum_machine, &loader, &mut ev_tracks);
    add_instrument_channel("muted_hit.ogg", &mut drum_machine, &loader, &mut ev_tracks);
}

#[derive(Default)]
struct LastTicks(HashMap<Entity, u64>);

fn playback_sys(
    assets: Res<Assets<StaticSoundAsset>>,
    channels: Query<(&KiraSoundHandle, &KiraAssociatedTracks, &Parent)>,
    patterns: Query<(Entity, &DrumPattern, &Parent)>,
    clock: Query<&KiraAssociatedClocks>,
    mut ev_play: EventWriter<PlaySoundEvent>,
    mut last_ticks: Local<LastTicks>,
) {
    for (pattern_id, pattern, parent) in patterns.iter() {
        if let Ok((sound, tracks, parent)) = channels.get(parent.get()) {
            let clock = clock.single().0.first().unwrap();
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

#[derive(Debug, Clone, Copy)]
enum Pallete {
    FreshGreen = 0x99dd55,
    LeafGreen = 0x44dd88,
    MintGreen = 0x22ccbb,
    AquaBlue = 0x0099cc,
    DeepBlue = 0x3366bb,
    GrapePurple = 0x663399,
}

impl From<Pallete> for Rgba {
    fn from(p: Pallete) -> Self {
        let col = p as u32;
        let r: u8 = ((col >> 16) & 0xff) as u8;
        let g: u8 = ((col >> 8) & 0xff) as u8;
        let b: u8 = (col & 0xff) as u8;
        Rgba::from_srgba_unmultiplied(r, g, b, 255)
    }
}

impl From<Pallete> for Color32 {
    fn from(p: Pallete) -> Self {
        let rgb: Rgba = p.into();
        rgb.into()
    }
}

const CHANNEL_UI_SIZES: [f32; 6] = [64.0, 64.0, 128.0, 128.0, 128.0, 128.0];
const MACHINE_H_PADDING: f32 = 32.0;

fn container_size_for_cells(sizes: &[f32], padding: f32) -> f32 {
    padding * (sizes.len() as f32 + 1.0) + sizes.iter().sum::<f32>()
}

fn ui_sys(
    mut ctx: EguiContexts,
    mut clocks: Query<&mut KiraAssociatedClocks>,
    mut channels: Query<(&KiraAssociatedTracks, &Children)>,
    mut bpm: Query<&mut Bpm>,
    mut patterns: Query<(Entity, &mut DrumPattern)>,
) {
    let mut first_clock = clocks.iter_mut().next();
    let clock = first_clock.as_mut().map(|c| &mut c.0[0]);
    egui::CentralPanel::default().show(ctx.ctx_mut(), |mut ui| {
        egui::warn_if_debug_build(ui);
        let padding = ui.spacing().item_spacing.x;
        StripBuilder::new(&mut ui)
            .size(Size::remainder())
            .size(Size::exact(
                container_size_for_cells(&CHANNEL_UI_SIZES, padding) + MACHINE_H_PADDING * 2.0,
            ))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();
                if let Some(clock) = clock {
                    let mut bpm = bpm.single_mut();
                    strip.cell(|ui| {
                        ui.label("BPM");
                        ui.add(
                            egui::Slider::new(&mut bpm.0, 20.0..=220.0)
                                .text("BPM")
                                .clamp_to_range(true),
                        );
                        let _ = clock.set_speed(
                            kira::ClockSpeed::TicksPerSecond(steps_per_sec(bpm.0)),
                            Tween::default(),
                        );
                        ui.separator();
                        machine_ui(ui, channels, patterns);
                    });
                }
                strip.empty();
            });
    });
}

fn machine_ui(
    mut ui: &mut egui::Ui,
    channels: Query<(&KiraAssociatedTracks, &Children)>,
    mut patterns: Query<(Entity, &mut DrumPattern)>,
) {
    let bg_color: Color32 = dark_color(Pallete::DeepBlue);
    ui.painter()
        .rect_filled(ui.available_rect_before_wrap(), 8.0, bg_color);
    ui.label("");
    let padding = ui.spacing().item_spacing.x;
    StripBuilder::new(&mut ui)
        .size(Size::remainder())
        .size(Size::exact(container_size_for_cells(
            &CHANNEL_UI_SIZES,
            padding,
        )))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.empty();
            strip.cell(|ui| {
                // Visit tracks patterns in order.
                for (channel_number, (mut tracks, children)) in channels.iter().enumerate() {
                    for (i, child) in children.iter().enumerate() {
                        if let Ok((_eid, mut pattern)) = patterns.get_mut(*child) {
                            channel_view(
                                ui,
                                channel_number,
                                "♪",
                                "drum",
                                0,
                                i,
                                &mut tracks,
                                &mut pattern,
                            );
                        }
                    }
                }
            });
            strip.empty();
        });
}

fn shift_color(color: impl Into<Rgba>, degrees: f32) -> Color32 {
    let mut color = Hsva::from(color.into());
    // H is between 0 and 1, so we need to multiply by 360 to get degrees.
    color.h = color.h + (degrees / 360.0).clamp(0.0, 1.0);
    color.into()
}

fn light_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s = color.s * 0.8;
    color.v = color.v * 0.70;
    color.into()
}

fn dark_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s = color.s * 0.35;
    color.v = color.v * 0.10;
    color.into()
}

fn channel_view(
    ui: &mut egui::Ui,
    channel_number: usize,
    icon: &str,
    title: &str,
    track_id: usize,
    pattern: usize,
    tracks: &KiraAssociatedTracks,
    drum_pattern: &mut DrumPattern,
) {
    StripBuilder::new(ui)
        .size(Size::exact(48.0))
        .vertical(|mut strip| {
            strip.strip(|mut builder| {
                for size in &CHANNEL_UI_SIZES {
                    builder = builder.size(Size::exact(*size));
                }
                builder.horizontal(|mut strip| {
                    strip.cell(|mut ui| {
                        channel_title_view(&mut ui, icon, title);
                    });
                    strip.cell(|mut ui| {
                        track_selector_view(&mut ui, track_id, tracks, drum_pattern);
                    });
                    let steps = &mut drum_pattern.steps[..];
                    for beat in 0..4 {
                        strip.cell(|mut ui| {
                            let base_color: Rgba = if beat % 2 == 0 {
                                shift_color(Pallete::FreshGreen, (channel_number + 1) as f32 * 30.0)
                                    .into()
                            } else {
                                shift_color(Pallete::FreshGreen, (channel_number + 1) as f32 * 40.0)
                                    .into()
                            };
                            let (_, tail) = steps.split_at_mut(beat * 4);
                            let (this_beat, _) = tail.split_array_mut();
                            beat_view(
                                &mut ui,
                                channel_number,
                                light_color(base_color),
                                dark_color(base_color),
                                pattern,
                                beat,
                                this_beat,
                            );
                        });
                    }
                });
            });
        });
}

fn channel_title_view(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.painter().rect_filled(
        ui.available_rect_before_wrap().shrink(1.0),
        4.0,
        dark_color(Pallete::LeafGreen),
    );
    ui.label(format!("{} {}", icon, title));
}

fn track_selector_view(
    ui: &mut egui::Ui,
    _track_id: usize,
    _tracks: &KiraAssociatedTracks,
    _drum_pattern: &mut DrumPattern,
) {
    ui.painter().rect_filled(
        ui.available_rect_before_wrap().shrink(1.0),
        4.0,
        dark_color(Pallete::AquaBlue),
    );
}

fn beat_view(
    ui: &mut egui::Ui,
    channel_num: usize,
    on_color: Color32,
    off_color: Color32,
    pattern: usize,
    beat: usize,
    steps: &mut [bool; 4],
) {
    ui.columns(4, |columns| {
        // debug!("beat_view: beat={}, steps={:?}", beat, steps);
        for (i, ui) in columns.iter_mut().enumerate() {
            let id = Id::new("drum_step").with((channel_num, pattern, beat, i));
            // dbg!(id);
            let target = ui.interact(ui.available_rect_before_wrap(), id, Sense::click());
            ui.painter().rect_filled(
                ui.available_rect_before_wrap().shrink(1.0),
                4.0,
                if steps[i] { on_color } else { off_color },
            );
            if target.clicked() {
                steps[i] = !steps[i];
            }
        }
        // debug!("beat_view end: beat={}, steps={:?}", beat, steps);
    });
    let rect = ui.available_rect_before_wrap();
    let r = rect.right() + 4.0;
    let t = rect.top() + 15.0;
    let b = rect.bottom() - 15.0;
    let right_top = Pos2::new(r, t);
    let right_bottom = Pos2::new(r, b);
    if beat < 3 {
        ui.painter().line_segment(
            [right_top, right_bottom],
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
    }
}
