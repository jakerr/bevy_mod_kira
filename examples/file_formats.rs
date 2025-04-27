use bevy::prelude::*;
use bevy_egui::{
    EguiContextPass, EguiContexts, EguiPlugin,
    egui::{self, Color32, RichText},
};
use bevy_mod_kira::{KiraPlaySoundEvent, KiraPlugin, KiraStaticSoundAsset, KiraStaticSoundHandle};

mod color_utils;
use color_utils::*;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy mod Kira - File Formats".into(),
                resolution: (300., 600.).into(),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            KiraPlugin,
            EguiPlugin {
                enable_multipass_for_primary_context: true,
            },
        ))
        .add_systems(Startup, setup_sys)
        .add_systems(EguiContextPass, ui_sys)
        .run();
}

#[derive(Component)]
struct FormatInfo {
    ext: String,
    base_color: Color32,
    file_name: String,
    enabled: bool,
    asset: Option<KiraStaticSoundHandle>,
}

#[derive(Component)]
struct AllFormats;

fn setup_sys(mut commands: Commands, loader: Res<AssetServer>) {
    let mut root = commands.spawn(AllFormats);
    let colors = [
        Pallete::AquaBlue,
        Pallete::DeepBlue,
        Pallete::MintGreen,
        Pallete::GrapePurple,
    ];
    let mut i = 0;
    for (ext, enabled) in [
        ("ogg", cfg!(feature = "ogg")),
        ("mp3", cfg!(feature = "mp3")),
        ("wav", cfg!(feature = "wav")),
        ("flac", cfg!(feature = "flac")),
    ] {
        let color = colors[i % colors.len()];
        let name = format!("say.{}", ext);
        let asset = if enabled {
            Some(KiraStaticSoundHandle(loader.load(name.clone())))
        } else {
            None
        };
        root.with_children(|parent| {
            parent.spawn(FormatInfo {
                ext: ext.into(),
                base_color: color.into(),
                file_name: name,
                enabled,
                asset,
            });
        });
        i += 1;
    }
}

fn ui_sys(
    mut ctx: EguiContexts,
    assets: Res<Assets<KiraStaticSoundAsset>>,
    formats: Query<&Children, With<AllFormats>>,
    query: Query<(Entity, &FormatInfo)>,
    mut ev_play: EventWriter<KiraPlaySoundEvent>,
) -> Result<(), BevyError> {
    let all_formats = formats.single()?;
    let ctx = ctx.try_ctx_mut();
    if ctx.is_none() {
        // Likely window is being closed on App exit.
        return Ok(());
    }
    egui::CentralPanel::default().show(ctx.unwrap(), |ui| {
        ui.vertical_centered_justified(|ui| {
            ui.label(RichText::from("File Formats").size(32.0));
            let mut some_disabled = false;
            for &format_id in all_formats {
                let (eid, info) = query.get(format_id).unwrap();
                let (name, bg_color, text_color) = if info.enabled {
                    let name = info.file_name.clone();
                    let bg = light_color(info.base_color);
                    (name, bg, contrasty(bg))
                } else {
                    some_disabled = true;
                    let name = format!("{} (not enabled)", info.ext);
                    let bg = Color32::DARK_GRAY;
                    (name, bg, Color32::GRAY)
                };
                let text = {
                    let r = RichText::from(name)
                        .size(24.0)
                        .background_color(bg_color)
                        .color(text_color);
                    if info.enabled { r } else { r.strikethrough() }
                };
                let button = egui::Button::new(text).fill(bg_color);
                let click = ui.add(button).clicked();
                if click {
                    debug!("clicked: {}", info.file_name);
                    if let Some(asset) = &info.asset {
                        if let Some(sound_asset) = assets.get(&asset.0) {
                            let sound_data = sound_asset.sound.clone();
                            let sound_event = KiraPlaySoundEvent::new(eid, None, sound_data);
                            ev_play.write(sound_event);
                        }
                    }
                }
            }
            if some_disabled {
                ui.separator();
                ui.label(
                    "Some formats are not enabled you can enable them with cargo feature \
                     flags such as --features=ogg,mp3,wav,flac.",
                );
            }
        });
    });
    Ok(())
}
