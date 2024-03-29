use std::ops::RangeInclusive;

use bevy::{ecs::system::EntityCommands, prelude::*, utils::HashMap};
use bevy_egui::{
    egui::{self, Pos2, Rgba, Stroke},
    EguiContexts, EguiPlugin,
};
use bevy_mod_kira::{
    KiraContext, KiraPlaySoundEvent, KiraPlugin, KiraStaticSoundAsset, KiraStaticSoundHandle,
};
use egui::{Color32, Id, RichText, Sense};
use egui_extras::{Size, StripBuilder};
use kira::{
    clock::ClockHandle,
    track::{
        effect::{filter::FilterHandle, reverb::ReverbHandle},
        TrackBuilder, TrackHandle, TrackId, TrackRoutes,
    },
    tween::Tween,
};

mod color_utils;
use color_utils::*;

const BPM: f64 = 90.0;
const BPS: f64 = BPM / 60.0;
const STEP_PER_BEAT: usize = 4;
const STEPS: usize = STEP_PER_BEAT * 4;
const STEP_PER_SEC: f64 = BPS * STEP_PER_BEAT as f64;

const CHANNEL_ROW_HEIGHT: f32 = 64.0;
const CHANNEL_UI_SIZES: [f32; 7] = [64.0, 18.0, 18.0, 128.0, 128.0, 128.0, 128.0];
const MACHINE_H_PADDING: f32 = 32.0;
const MACHINE_V_PADDING: f32 = 12.0;

fn steps_per_sec(bpm: f64) -> f64 {
    bpm / 60.0 * STEP_PER_BEAT as f64
}

// 16 bit boolean constants are a convienient way to represent 16 step patterns when defining
// defaults. But for use in the UI it's more convienient to hold them as an array of bools in the
// component so click handlers can just be given one &mut bool to flip.
struct DefaultPattern(u16);
const DEFAULT_KICK: DefaultPattern = DefaultPattern(0b1001_0000_1001_0000);
const DEFAULT_HAT: DefaultPattern = DefaultPattern(0b0010_1010_1010_1111);
const DEFAULT_SNARE: DefaultPattern = DefaultPattern(0b0000_1000_0000_1000);
const DEFAULT_HIT: DefaultPattern = DefaultPattern(0b0010_0000_1101_0101);

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

#[derive(Component, Default)]
struct DrumPattern {
    steps: [bool; STEPS],
}

#[derive(Component)]
struct Bpm(f64);

#[derive(Component)]
struct MainClock(ClockHandle);

#[derive(Component)]
struct ChannelTrack(TrackHandle);

#[derive(Component)]
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
                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((KiraPlugin, EguiPlugin))
        .add_systems(Startup, setup_sys)
        .add_systems(Update, (playback_sys, apply_levels_sys, ui_sys))
        .run();
}

#[derive(Component)]
struct TrackReverb(ReverbHandle);

#[derive(Component)]
// We'll store a float for our filter cutoff setting so it's easy to map to an egui slider and then
// apply in the apply_levels_sys.
struct MainFilter(FilterHandle, f64);

#[derive(Component)]
struct DrumMachine; // Tag component

//
// Systems
//

fn setup_sys(mut commands: Commands, mut kira: NonSendMut<KiraContext>, loader: Res<AssetServer>) {
    // Create a top level entity to hold settings relevant to playback.
    let mut drum_machine = commands.spawn(DrumMachine);
    drum_machine.insert(Bpm(BPM));
    // This tells Kira to add a new clock and associate it with the drum machine entity.
    // Clock handles will be added to the KiraClocks component on that entity.
    let clock_handle = kira
        .add_clock(kira::clock::ClockSpeed::TicksPerSecond(STEP_PER_SEC))
        .expect("Failed to create clock.");
    clock_handle.start().expect("Failed to start clock");
    drum_machine.insert(MainClock(clock_handle));

    // Create a track to hold the filter effect we'll route every channel's individual track through
    // this one shared filter.
    let filter = kira::track::effect::filter::FilterBuilder::new().cutoff(440.0);
    let mut filter_track_builder = TrackBuilder::new();
    let filter_handle = filter_track_builder.add_effect(filter);
    let filter_track_handle = kira
        .add_track(filter_track_builder)
        .expect("Failed to create filter track.");
    let filter_track_id = filter_track_handle.id();
    drum_machine.insert(ChannelTrack(filter_track_handle));
    drum_machine.insert(MainFilter(filter_handle, 0.5));

    add_instrument_channel(
        "kick.ogg",
        "♡",
        DEFAULT_KICK,
        false,
        &mut drum_machine,
        &loader,
        &mut kira,
        dbg!(filter_track_id),
    );
    add_instrument_channel(
        "hat.ogg",
        "☀",
        DEFAULT_HAT,
        false,
        &mut drum_machine,
        &loader,
        &mut kira,
        filter_track_id,
    );
    add_instrument_channel(
        "snare.ogg",
        "⛃",
        DEFAULT_SNARE,
        false,
        &mut drum_machine,
        &loader,
        &mut kira,
        filter_track_id,
    );
    add_instrument_channel(
        "hit.ogg",
        "🔘",
        DEFAULT_HIT,
        true,
        &mut drum_machine,
        &loader,
        &mut kira,
        filter_track_id,
    );
}

#[derive(Default)]
struct LastTicks(HashMap<Entity, u64>);

fn playback_sys(
    assets: Res<Assets<KiraStaticSoundAsset>>,
    channels: Query<(Entity, &KiraStaticSoundHandle, &ChannelTrack, &DrumPattern)>,
    clock: Query<&MainClock>,
    mut ev_play: EventWriter<KiraPlaySoundEvent>,
    mut last_ticks: Local<LastTicks>,
) {
    for (chan_id, sound, track, pattern) in channels.iter() {
        let clock = &clock.single().0;
        let clock_ticks = clock.time().ticks;
        let last_tick = last_ticks.0.get(&chan_id).copied().unwrap_or(u64::MAX);
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
        last_ticks.0.insert(chan_id, clock_ticks);
        let next_play_step = clock_ticks as usize % STEPS;
        if pattern.steps[next_play_step] {
            if let Some(sound_asset) = assets.get(&sound.0) {
                let sound = sound_asset.sound.0.with_modified_settings(|mut settings| {
                    // We calculate next_play_step as the the step at current clock time, but we
                    // want to start the sound right at precise tick so every sound will be
                    // triggered at a 1 tick offset.
                    settings = settings
                        .output_destination(&track.0)
                        .start_time(clock.time() + 1);
                    settings
                });

                // When we send the play sound event we pass the entity id for the channel. This
                // instructs the KiraPlugin to associate the playing sound handle with this entity by
                // inserting a KiraActiveSounds component. That component can be used to adjust various
                // aspects of the playing sound at runtime. It will automatically be removed when all
                // sounds associated with the entity have finished playing.
                ev_play.send(KiraPlaySoundEvent::new(chan_id, sound));
            } else {
                warn!(
                    "Sound asset not found for handle: {:?} it may still be loading.",
                    sound.0
                );
            }
        }
    }
}

fn apply_levels_sys(
    mut channels: Query<(&ChannelInfo, &mut ChannelTrack, &mut TrackReverb), Changed<ChannelInfo>>,
    mut filter: Query<&mut MainFilter>,
) {
    for (info, track, mut reverb) in channels.iter_mut() {
        let volume = if info.muted { 0.0 } else { info.volume };
        let _ = track.0.set_volume(volume, Tween::default());
        let _ = reverb.0.set_mix(info.reverb, Tween::default());
    }
    for mut filter in filter.iter_mut() {
        let value = filter.1;
        let _ = filter.0.set_mix(value, Tween::default());
    }
}

fn ui_sys(
    mut ctx: EguiContexts,
    clock: Query<&MainClock>,
    channel_ids: Query<&Children, With<DrumMachine>>,
    channels: Query<(Entity, &mut ChannelTrack, &mut DrumPattern)>,
    chan_mute: Query<&mut ChannelInfo>,
    mut bpm: Query<&mut Bpm>,
    mut filter: Query<&mut MainFilter>,
) {
    let clock = &clock.single().0;
    egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {
        egui::warn_if_debug_build(ui);
        let padding = ui.spacing().item_spacing.x;
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(
                container_size_for_cells(&CHANNEL_UI_SIZES, padding) + MACHINE_H_PADDING * 2.0,
            ))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();
                let mut bpm = bpm.single_mut();
                let mut filter = filter.single_mut();
                strip.cell(|ui| {
                    let _ = clock.set_speed(
                        kira::clock::ClockSpeed::TicksPerSecond(steps_per_sec(bpm.0)),
                        Tween::default(),
                    );
                    machine_ui(ui, &mut bpm, &mut filter, channel_ids, channels, chan_mute);
                });
                strip.empty();
            });
    });
}

//
// Private utility functions
//

fn add_instrument_channel(
    asset: &'static str,
    icon: &str,
    default_pattern: impl Into<DrumPattern>,
    default_mute: bool,
    parent: &mut EntityCommands,
    loader: &AssetServer,
    kira: &mut KiraContext,
    filter_track_id: TrackId,
) {
    // The parent passed in here is the drum_machine entity from the setup_sys function.
    // We are adding a child entity to the drum_machine entity for each instrument channel.
    parent.with_children(|parent| {
        let a = loader.load(asset);
        let mut channel = parent.spawn(KiraStaticSoundHandle(a));
        let name = asset.split('.').next().unwrap();

        // This ChannelInfo component is defined specifically for this demo. It is used to hold the
        // channel state to show in the UI and to hold the volume level and mute status of the
        // channel which will be applied every frame by the apply_levels_sys system.
        channel.insert(ChannelInfo {
            name: name.to_string(),
            icon: icon.to_string(),
            muted: default_mute,
            ..Default::default()
        });

        // Next we add a track to the channel and adding a reverb effect to the track. Both of these
        // steps are optional. If you don't specify a track when playing a sound it will play on
        // a default Main track.
        let reverb = kira::track::effect::reverb::ReverbBuilder::new()
            .mix(0.0)
            .stereo_width(0.0);
        let volume = if default_mute { 0.0 } else { 1.0 };
        let mut track = TrackBuilder::new()
            .volume(volume)
            .routes(TrackRoutes::parent(filter_track_id));

        // The reverb handle is returned directly from the track builder even before we've sent it
        // to Kira so it's our responsibility to hold onto it in a component if we want to be able
        // to modify it later.
        let reverb_handle = track.add_effect(reverb);
        channel.insert(TrackReverb(reverb_handle));

        // We send the track builder to Kira along with the entity id for this channel. Once added
        // the KiraPlugin will add the track to KiraTracks component on the channel entity.
        let track_handle = kira.add_track(track).expect("Failed to add track to Kira.");
        channel.insert(ChannelTrack(track_handle));

        // Finally we insert the default pattern for this channel.
        channel.insert(default_pattern.into());
    });
}

fn container_size_for_cells(sizes: &[f32], padding: f32) -> f32 {
    padding * (sizes.len() - 1) as f32 + sizes.iter().sum::<f32>()
}

//
// UI elements
//

fn machine_ui(
    ui: &mut egui::Ui,
    bpm: &mut Bpm,
    filter: &mut MainFilter,
    // Used to draw the channels in the correct order.
    channel_ids: Query<&Children, With<DrumMachine>>,
    mut channels: Query<(Entity, &mut ChannelTrack, &mut DrumPattern)>,
    mut chan_mute: Query<&mut ChannelInfo>,
) {
    let padding_x = ui.spacing().item_spacing.x;
    let padding_y = ui.spacing().item_spacing.y;
    let total_height = (CHANNEL_ROW_HEIGHT + padding_y) * 5.0 + MACHINE_V_PADDING * 2.0;
    let bg_color: Color32 = dark_color(Pallete::DeepBlue);
    StripBuilder::new(ui)
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
                            ui.add(
                                egui::Slider::new(&mut filter.1, 0.0..=1.0)
                                    .text("Filter")
                                    .clamp_to_range(true),
                            );
                            ui.add_space(10.0);
                            // Visit channels in order of the drum machine container.
                            let chan_ids = channel_ids.single();
                            let mut in_order = channels.iter_many_mut(chan_ids);

                            let mut chan_number = 0;
                            while let Some((chan_id, mut track, mut pattern)) =
                                in_order.fetch_next()
                            {
                                let mut chan_mut = chan_mute.get_mut(chan_id).unwrap();
                                channel_view(
                                    ui,
                                    chan_number,
                                    &mut chan_mut,
                                    &mut track.0,
                                    &mut pattern,
                                );
                                chan_number += 1;
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
    channel_number: u32,
    info: &mut ChannelInfo,
    track: &mut TrackHandle,
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
                    strip.cell(|ui| {
                        channel_title_view(ui, base_color, track, info);
                    });
                    let is_muted = info.muted;
                    strip.cell(|ui| {
                        track_fader_view(
                            ui,
                            Pallete::LeafGreen,
                            &mut info.volume,
                            1.0..=0.0,
                            is_muted,
                        );
                    });
                    strip.cell(|ui| {
                        track_fader_view(
                            ui,
                            Pallete::DeepBlue,
                            &mut info.reverb,
                            0.5..=0.0,
                            is_muted,
                        );
                    });
                    let steps = &mut drum_pattern.steps[..];
                    for beat in 0..4 {
                        strip.cell(|ui| {
                            let mut beat_color = base_color;
                            if beat % 2 == 1 {
                                beat_color =
                                    shift_color(beat_color, (channel_number + 1) as f32 * 12.0)
                                        .into();
                            };
                            let this_beat = &mut steps[beat * 4..(beat + 1) * 4];
                            beat_view(
                                ui,
                                channel_number,
                                if info.muted {
                                    muted_color(beat_color)
                                } else {
                                    light_color(beat_color)
                                },
                                dark_color(beat_color),
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
        CHANNEL_UI_SIZES[0],
        CHANNEL_UI_SIZES[1],
        CHANNEL_UI_SIZES[2],
        (CHANNEL_UI_SIZES[3] + padding_x) * 4.0,
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

    let mut color = color.into();
    let full_color: Rgba = color;
    let mute_color: Rgba = muted_color(color).into();
    if is_muted {
        color = mute_color;
    } else {
        let v = *value as f32;
        let start = *range.start() as f32;
        let end = *range.end() as f32;
        let a = start.min(end);
        let b = start.max(end);
        let color_sat = v / (b - a);
        color = egui::lerp(mute_color..=full_color, color_sat);
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
    channel_num: u32,
    on_color: Color32,
    off_color: Color32,
    beat: usize,
    steps: &mut [bool],
) {
    ui.columns(4, |columns| {
        for (i, ui) in columns.iter_mut().enumerate() {
            let id = Id::new("drum_step").with((channel_num, beat, i));
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
