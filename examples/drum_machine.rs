#![feature(split_array)]

use std::ops::RangeInclusive;

use bevy::{
    ecs::system::EntityCommands,
    prelude::{
        default, error, warn, App, AssetServer, Assets, BuildChildren, Changed, Children, Commands,
        Component, Entity, EventWriter, Local, Parent, PluginGroup, Query, Res,
    },
    reflect::Reflect,
    utils::HashMap,
    window::{PresentMode, Window, WindowPlugin},
    DefaultPlugins,
};
use bevy_egui::{
    egui::{self, epaint::Hsva, Pos2, Rgba, Stroke},
    EguiContexts, EguiPlugin,
};
use bevy_mod_kira::{
    AddClockEvent, AddTrackEvent, KiraAssociatedClocks, KiraAssociatedTracks, KiraPlugin,
    KiraSoundHandle, PlaySoundEvent, StaticSoundAsset,
};
use egui::{Color32, Id, RichText, Sense};
use egui_extras::{Size, StripBuilder};
use kira::{
    track::{effect::reverb::ReverbHandle, TrackBuilder, TrackHandle},
    tween::Tween,
};

const BPM: f64 = 90.0;
const BPS: f64 = BPM / 60.0;
const STEP_PER_BEAT: usize = 4;
const STEPS: usize = STEP_PER_BEAT * 4;
const STEP_PER_SEC: f64 = BPS * STEP_PER_BEAT as f64;

struct DefaultPattern(u16);
const DEFAULT_KICK: DefaultPattern = DefaultPattern(0b1001_0000_1001_0000);
const DEFAULT_HAT: DefaultPattern = DefaultPattern(0b0010_1010_1010_1111);
const DEFAULT_SNARE: DefaultPattern = DefaultPattern(0b0000_1000_0000_1000);
const DEFAULT_HIT: DefaultPattern = DefaultPattern(0b0010_0000_1101_0101);

const CHANNEL_ROW_HEIGHT: f32 = 64.0;
const CHANNEL_UI_SIZES: [f32; 7] = [64.0, 18.0, 18.0, 128.0, 128.0, 128.0, 128.0];
const MACHINE_H_PADDING: f32 = 32.0;
const MACHINE_V_PADDING: f32 = 6.0;

fn steps_per_sec(bpm: f64) -> f64 {
    bpm / 60.0 * STEP_PER_BEAT as f64
}

impl From<DefaultPattern> for DrumPattern {
    fn from(p: DefaultPattern) -> Self {
        let p = p.0.reverse_bits();
        let mut steps = [false; STEPS];
        for i in 0..STEPS {
            steps[i] = (p & (1 << i)) != 0;
        }
        Self { steps }
    }
}

#[derive(Component, Default, Reflect)]
struct DrumPattern {
    steps: [bool; STEPS],
}

#[derive(Component, Reflect)]
struct Bpm(f64);

#[derive(Component, Reflect)]
struct ChannelInfo {
    name: String,
    muted: bool,
    icon: String,
    volume: f64,
    reverb: f64,
}

impl Default for ChannelInfo {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            muted: false,
            icon: "🔊".to_string(),
            volume: 0.8,
            reverb: 0.0,
        }
    }
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy mod Kira - Drum Machine".into(),
                resolution: (800., 460.).into(),
                present_mode: PresentMode::AutoVsync,
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
        .add_system(playback_sys)
        .add_system(apply_levels_sys)
        .add_system(ui_sys)
        .register_type::<TrackReverb>()
        .run();
}

#[derive(Component, Reflect)]
struct TrackReverb(#[reflect(ignore)] ReverbHandle);

//
// Systems
//

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
    add_instrument_channel(
        "kick.ogg",
        "♡",
        DEFAULT_KICK,
        false,
        &mut drum_machine,
        &loader,
        &mut ev_tracks,
    );
    add_instrument_channel(
        "hat.ogg",
        "☀",
        DEFAULT_HAT,
        false,
        &mut drum_machine,
        &loader,
        &mut ev_tracks,
    );
    add_instrument_channel(
        "snare.ogg",
        "⛃",
        DEFAULT_SNARE,
        false,
        &mut drum_machine,
        &loader,
        &mut ev_tracks,
    );
    add_instrument_channel(
        "hit.ogg",
        "🔘",
        DEFAULT_HIT,
        true,
        &mut drum_machine,
        &loader,
        &mut ev_tracks,
    );
}

#[derive(Default)]
struct LastTicks(HashMap<Entity, u64>);

fn playback_sys(
    assets: Res<Assets<StaticSoundAsset>>,
    channels: Query<(&KiraSoundHandle, &KiraAssociatedTracks)>,
    patterns: Query<(Entity, &DrumPattern, &Parent)>,
    clock: Query<&KiraAssociatedClocks>,
    mut ev_play: EventWriter<PlaySoundEvent>,
    mut last_ticks: Local<LastTicks>,
) {
    for (pattern_id, pattern, parent) in patterns.iter() {
        if let Ok((sound, tracks)) = channels.get(parent.get()) {
            let clock = clock.single().0.first().unwrap();
            let clock_ticks = clock.time().ticks;
            let last_tick = last_ticks.0.get(&pattern_id).copied().unwrap_or(u64::MAX);
            if clock_ticks == last_tick {
                continue;
            }
            if (clock_ticks - last_tick) > 1 {
                // Playback system is not running at the same speed as the clock. Sometimes this
                // system can miss ticks. This seems to happen often when backgrounding the app.
                // Since this is a demo it's not imperative that every beat is scheduled but if this
                // was a real instrument we could work around this by queueing up more than one tick
                // in advance. That adds complexity around making sure we don't double schedule
                // a step so we'll just warn for this demo. In practice I haven't seen it miss
                // a tick when the app is forgrounded.
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

fn apply_levels_sys(
    mut channels: Query<
        (&ChannelInfo, &mut KiraAssociatedTracks, &mut TrackReverb),
        Changed<ChannelInfo>,
    >,
) {
    for (info, mut tracks, mut reverb) in channels.iter_mut() {
        for track in tracks.0.iter_mut() {
            let volume = if info.muted { 0.0 } else { info.volume };
            let _ = track.set_volume(volume, Tween::default());
        }
        let _ = reverb.0.set_mix(info.reverb, Tween::default());
    }
}

fn ui_sys(
    mut ctx: EguiContexts,
    mut clocks: Query<&mut KiraAssociatedClocks>,
    channels: Query<(Entity, &mut KiraAssociatedTracks, &Children)>,
    chan_mute: Query<&mut ChannelInfo>,
    mut bpm: Query<&mut Bpm>,
    patterns: Query<(Entity, &mut DrumPattern)>,
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
                        let _ = clock.set_speed(
                            kira::ClockSpeed::TicksPerSecond(steps_per_sec(bpm.0)),
                            Tween::default(),
                        );
                        machine_ui(ui, &mut bpm, channels, chan_mute, patterns);
                    });
                }
                strip.empty();
            });
    });
}

//
// Private utility functions
//

fn add_instrument_channel(
    asset: &str,
    icon: &str,
    default_pattern: impl Into<DrumPattern>,
    default_mute: bool,
    parent: &mut EntityCommands,
    loader: &AssetServer,
    ev_tracks: &mut EventWriter<AddTrackEvent>,
) {
    let a = loader.load(asset);
    parent.with_children(|parent| {
        let mut channel = parent.spawn(KiraSoundHandle(a));
        let name = asset.split('.').next().unwrap();
        channel.insert(ChannelInfo {
            name: name.to_string(),
            icon: icon.to_string(),
            muted: default_mute,
            ..Default::default()
        });
        let reverb = kira::track::effect::reverb::ReverbBuilder::new()
            .mix(0.0)
            .stereo_width(0.0);
        let volume = if default_mute { 0.0 } else { 1.0 };
        let mut track = TrackBuilder::new().volume(volume);
        let reverb_handle = track.add_effect(reverb);
        ev_tracks.send(AddTrackEvent::new(channel.id(), track));
        channel.insert(TrackReverb(reverb_handle));
        channel.with_children(|parent| {
            parent.spawn(default_pattern.into());
        });
    });
}

fn container_size_for_cells(sizes: &[f32], padding: f32) -> f32 {
    padding * (sizes.len() - 1) as f32 + sizes.iter().sum::<f32>()
}

//
// UI elements
//

fn machine_ui(
    mut ui: &mut egui::Ui,
    bpm: &mut Bpm,
    mut channels: Query<(Entity, &mut KiraAssociatedTracks, &Children)>,
    mut chan_mute: Query<&mut ChannelInfo>,
    mut patterns: Query<(Entity, &mut DrumPattern)>,
) {
    let padding_x = ui.spacing().item_spacing.x;
    let padding_y = ui.spacing().item_spacing.y;
    let total_height = (CHANNEL_ROW_HEIGHT + padding_y) * 5.0 + MACHINE_V_PADDING * 2.0;
    let bg_color: Color32 = dark_color(Pallete::DeepBlue);
    StripBuilder::new(&mut ui)
        .size(Size::remainder())
        .size(Size::exact(total_height))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.empty();
            strip.strip(|builder| {
                builder
                    .size(Size::remainder())
                    .size(Size::exact(container_size_for_cells(
                        &CHANNEL_UI_SIZES,
                        padding_x,
                    )))
                    .size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.empty();
                        strip.cell(|ui| {
                            let paint_rect = ui.available_rect_before_wrap();
                            let paint_rect = paint_rect.shrink(-MACHINE_H_PADDING);
                            ui.painter().rect_filled(paint_rect, 8.0, bg_color);
                            ui.add(
                                egui::Slider::new(&mut bpm.0, 20.0..=220.0)
                                    .text("BPM")
                                    .clamp_to_range(true),
                            );
                            ui.add_space(10.0);
                            // Visit tracks patterns in order.
                            for (channel_number, (chan_id, mut tracks, children)) in
                                channels.iter_mut().enumerate()
                            {
                                for (i, child) in children.iter().enumerate() {
                                    if let Ok((_eid, mut pattern)) = patterns.get_mut(*child) {
                                        let mut chan_mut = chan_mute.get_mut(chan_id).unwrap();
                                        channel_view(
                                            ui,
                                            channel_number,
                                            &mut chan_mut,
                                            0,
                                            i,
                                            &mut tracks,
                                            &mut pattern,
                                        );
                                    }
                                }
                            }
                            control_legend_view(ui);
                        });
                        strip.empty();
                    });
            });
            strip.empty();
        });
}

fn channel_view(
    ui: &mut egui::Ui,
    channel_number: usize,
    info: &mut ChannelInfo,
    track_id: usize,
    pattern: usize,
    tracks: &mut KiraAssociatedTracks,
    drum_pattern: &mut DrumPattern,
) {
    StripBuilder::new(ui)
        .size(Size::exact(CHANNEL_ROW_HEIGHT))
        .vertical(|mut strip| {
            strip.strip(|mut builder| {
                for size in &CHANNEL_UI_SIZES {
                    builder = builder.size(Size::exact(*size));
                }
                builder.horizontal(|mut strip| {
                    let base_color: Rgba =
                        shift_color(Pallete::FreshGreen, (channel_number + 1) as f32 * 30.0).into();
                    strip.cell(|mut ui| {
                        let mut track = tracks.0.get_mut(track_id).unwrap();
                        channel_title_view(&mut ui, base_color, &mut track, info);
                    });
                    let is_muted = info.muted;
                    strip.cell(|mut ui| {
                        track_fader_view(
                            &mut ui,
                            Pallete::LeafGreen,
                            &mut info.volume,
                            0.0..=1.0,
                            is_muted,
                        );
                    });
                    strip.cell(|mut ui| {
                        track_fader_view(
                            &mut ui,
                            Pallete::DeepBlue,
                            &mut info.reverb,
                            0.0..=0.5,
                            is_muted,
                        );
                    });
                    let steps = &mut drum_pattern.steps[..];
                    for beat in 0..4 {
                        strip.cell(|mut ui| {
                            let mut beat_color = base_color;
                            if beat % 2 == 1 {
                                beat_color =
                                    shift_color(beat_color, (channel_number + 1) as f32 * 12.0)
                                        .into();
                            };
                            let (_, tail) = steps.split_at_mut(beat * 4);
                            let (this_beat, _) = tail.split_array_mut();
                            beat_view(
                                &mut ui,
                                channel_number,
                                if info.muted {
                                    muted_color(beat_color)
                                } else {
                                    light_color(beat_color)
                                },
                                dark_color(beat_color),
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

fn control_legend_view(ui: &mut egui::Ui) {
    let padding_x = ui.spacing().item_spacing.x;
    let sizes = [
        *&CHANNEL_UI_SIZES[0],
        *&CHANNEL_UI_SIZES[1],
        *&CHANNEL_UI_SIZES[2],
        (*&CHANNEL_UI_SIZES[3] + padding_x) * 4.0,
    ];
    StripBuilder::new(ui)
        .size(Size::exact(32.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.strip(|mut builder| {
                for size in sizes {
                    builder = builder.size(Size::exact(size));
                }
                builder.horizontal(|mut strip| {
                    strip.cell(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label("mute");
                        });
                    });
                    strip.cell(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label("v");
                        });
                    });
                    strip.cell(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label("r");
                        });
                    });
                    strip.cell(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label("steps");
                        });
                    });
                });
            });
            strip.cell(|ui| {
                ui.label("      v: volume");
                ui.label("      r: reverb");
            });
        });
}

fn channel_title_view(
    ui: &mut egui::Ui,
    mut color: Rgba,
    track: &mut TrackHandle,
    info: &mut ChannelInfo,
) {
    let rect = ui.available_rect_before_wrap().shrink(1.0);
    let id = Id::new("channel_title").with(track.id());
    let touch = ui.interact(rect, id, Sense::click());
    color = if info.muted {
        dark_color(color).into()
    } else {
        light_color(color).into()
    };
    ui.painter().rect_filled(rect, 4.0, color);
    ui.centered_and_justified(|ui| {
        let text = format!("{}\n{}", &info.name, &info.icon);
        let text = RichText::new(text).color(contrasty(color));
        ui.label(text);
    });
    if touch.clicked() {
        info.muted = !info.muted;
    }
}

fn track_fader_view(
    ui: &mut egui::Ui,
    color: impl Into<Rgba>,
    value: &mut f64,
    range: RangeInclusive<f64>,
    is_muted: bool,
) {
    let height = ui.available_height();
    let spacing = ui.spacing_mut();
    spacing.slider_width = height - 6.0;
    let style = ui.style_mut();

    let mut color = color.into().clone();
    let full_color: Rgba = color;
    let mute_color: Rgba = muted_color(color).into();
    if is_muted {
        color = mute_color;
    } else {
        let v = *value as f32;
        let color_sat: f32 = v / (*range.end() as f32 - *range.start() as f32);
        color = egui::lerp(mute_color..=full_color, color_sat).into();
    }

    let color = color.into();
    style.visuals.widgets.inactive.bg_fill = color;
    style.visuals.widgets.active.bg_fill = color;
    style.visuals.widgets.hovered.bg_fill = color;
    ui.add_space(3.0);
    ui.add(
        egui::Slider::new(value, range)
            .vertical()
            .show_value(false)
            .clamp_to_range(true),
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
        for (i, ui) in columns.iter_mut().enumerate() {
            let id = Id::new("drum_step").with((channel_num, pattern, beat, i));
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

//
// Color utils
//

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
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

fn shift_color(color: impl Into<Rgba>, degrees: f32) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.h = color.h + (degrees / 360.0);
    if color.h > 1.0 {
        color.h = color.h - 1.0;
    }
    color.into()
}

fn light_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s = color.s * 0.8;
    color.v = color.v * 0.70;
    color.into()
}

fn muted_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s = color.s * 0.35;
    color.v = color.v * 0.30;
    color.into()
}

fn dark_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s = color.s * 0.35;
    color.v = color.v * 0.10;
    color.into()
}

fn contrasty(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    let brightness = color.v * color.s;
    color.v = if brightness > 0.3 { 0.2 } else { 0.8 };
    color.into()
}
