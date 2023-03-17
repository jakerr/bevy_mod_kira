use std::fmt::Debug;
use std::fmt::Formatter;

use bevy::prelude::Events;

use bevy::prelude::error;
use bevy::prelude::EventReader;
use bevy::reflect::Reflect;
use bevy::{
    app::Plugin,
    prelude::{Commands, Component, Entity, Query, ResMut},
};
use kira::sound::static_sound::PlaybackState;
use kira::sound::SoundData;

use kira::track::TrackBuilder;
use kira::track::TrackHandle;

pub use crate::static_sound_loader::{StaticSoundAsset, StaticSoundFileLoader};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};

use crate::KiraContext;

#[derive(Component, Default, Reflect)]
pub struct KiraActiveSounds(#[reflect(ignore)] pub Vec<StaticSoundHandle>);

#[derive(Component, Default, Reflect)]
pub struct KiraAssociatedTracks(#[reflect(ignore)] pub Vec<TrackHandle>);

pub struct PlaySoundEvent<D: SoundData = StaticSoundData> {
    entity: Entity,
    sound: D,
}

pub struct AddTrackEvent {
    entity: Entity,
    track: TrackBuilder,
}

impl PlaySoundEvent {
    pub fn new(entity: Entity, sound: StaticSoundData) -> Self {
        Self { entity, sound }
    }
}

impl AddTrackEvent {
    pub fn new(entity: Entity, track: TrackBuilder) -> Self {
        Self { entity, track }
    }
}

impl Debug for KiraActiveSounds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KiraActiveSounds")
            .field("len", &self.0.len())
            .finish()
    }
}

pub struct KiraEventsPlugin;

impl Plugin for KiraEventsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_event::<PlaySoundEvent<StaticSoundData>>()
            // Track add events will not have automatic cleanup we need to manually consume them to
            // take the internal track out of the event.
            .init_resource::<Events<AddTrackEvent>>()
            .add_system(do_play_sys)
            .add_system(do_add_track_sys)
            .add_system(cleanup_inactive_sounds_sys)
            .register_type::<KiraActiveSounds>();
    }
}

fn do_play_sys(
    mut commands: Commands,
    mut kira: ResMut<KiraContext>,
    mut query: Query<(Entity, Option<&mut KiraActiveSounds>)>,
    mut ev_play: EventReader<PlaySoundEvent>,
) {
    for event in ev_play.iter() {
        let sound_handle = match kira.play(event.sound.clone()) {
            Ok(s) => s,
            Err(e) => {
                error!("Error playing sound: {}", e);
                continue;
            }
        };
        if let Ok((eid, active_sounds)) = query.get_mut(event.entity) {
            match active_sounds {
                Some(mut sounds) => {
                    sounds.0.push(sound_handle);
                }
                None => {
                    commands
                        .entity(eid)
                        .insert(KiraActiveSounds(vec![sound_handle]));
                }
            };
        } else {
            error!(
                "Failed to associate playing sound handle with entity: {:?}. \
                 The handle will be dropped.",
                event.entity
            );
        }
    }
}

fn do_add_track_sys(
    mut commands: Commands,
    mut kira: ResMut<KiraContext>,
    mut query: Query<(Entity, Option<&mut KiraAssociatedTracks>)>,
    mut ev_add_track: ResMut<Events<AddTrackEvent>>,
) {
    // extract events so that we can take the track out of the event.
    // let events = ev_add_track.iter().collect::<Vec<_>>();
    for event in ev_add_track.drain() {
        if let Some(manager) = kira.get_manager() {
            if let Ok(track_handle) = manager.add_sub_track(event.track) {
                if let Ok((eid, tracks)) = query.get_mut(event.entity) {
                    match tracks {
                        Some(mut sounds) => {
                            sounds.0.push(track_handle);
                        }
                        None => {
                            commands
                                .entity(eid)
                                .insert(KiraAssociatedTracks(vec![track_handle]));
                        }
                    };
                } else {
                    error!(
                        "Failed to associate playing sound handle with entity: {:?}. \
                 The handle will be dropped.",
                        event.entity
                    );
                }
            }
        }
    }
}

fn cleanup_inactive_sounds_sys(
    mut commands: Commands,
    mut query: Query<(Entity, &mut KiraActiveSounds)>,
) {
    for (eid, mut sounds) in query.iter_mut() {
        // first check for at least one stopped sound before deref mut to avoid spurious change
        // notifications notification. This is not yet profiled so may be a premature optimization.
        // note that `any` is short-circuiting so we don't need to worry about the cost iterating
        // over every sound.
        let needs_cleanup = sounds
            .0
            .iter()
            .any(|sound| sound.state() == PlaybackState::Stopped);

        if needs_cleanup {
            sounds
                .0
                .retain(|sound| sound.state() != PlaybackState::Stopped);
        }
        if sounds.0.is_empty() {
            commands.entity(eid).remove::<KiraActiveSounds>();
        }
    }
}
