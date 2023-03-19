use bevy::prelude::Events;

use bevy::prelude::error;
use bevy::prelude::{Commands, Component, Entity, Query, ResMut};
use bevy::reflect::Reflect;

use kira::track::TrackBuilder;
use kira::track::TrackHandle;

pub use crate::static_sound_loader::{StaticSoundAsset, StaticSoundFileLoader};

use crate::KiraContext;

#[derive(Component, Default, Reflect)]
pub struct KiraAssociatedTracks(#[reflect(ignore)] pub Vec<TrackHandle>);

pub struct AddTrackEvent {
    entity: Entity,
    track: TrackBuilder,
}

impl AddTrackEvent {
    pub fn new(entity: Entity, track: TrackBuilder) -> Self {
        Self { entity, track }
    }
}

pub(super) fn do_add_track_sys(
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