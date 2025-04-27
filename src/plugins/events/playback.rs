use std::fmt::Debug;
use std::fmt::Formatter;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::Mutex;

use bevy::prelude::*;
use kira::sound::PlaybackState;
use kira::track;
use kira::track::TrackHandle;

use crate::DynamicSoundHandle;
use crate::KiraPlayable;
pub use crate::sound::sound_types::KiraPlayingSound;
use crate::sound::sound_types::KiraTrackHandle;
use kira::sound::static_sound::StaticSoundHandle;

use crate::KiraContext;

#[derive(Component, Default, Reflect)]
/// This Component represents a collection of all currently playing sounds for an entity.
/// The sounds can be iterated over using the `static_handles` and `dynamic_handles` methods.
pub struct KiraPlayingSounds(#[reflect(ignore)] pub(crate) Vec<KiraPlayingSound>);

impl KiraPlayingSounds {
    /// Returns an iterator over all currently playing static sounds' [`StaticSoundHandle`]s.
    ///
    /// [`StaticSoundHandle`]: https://docs.rs/kira/latest/kira/sound/static_sound/struct.StaticSoundHandle.html
    pub fn static_handles(&self) -> impl Iterator<Item = &StaticSoundHandle> {
        self.0.iter().filter_map(|sound| match sound {
            KiraPlayingSound::Static(sound) => Some(sound),
            KiraPlayingSound::Dynamic(_) => None,
        })
    }
    /// Returns an iterator over all currently playing dynamic sound [`DynamicSoundHandle`] handles
    /// for the specified concrete type `T`.
    pub fn dynamic_handles<T>(&self) -> impl Iterator<Item = &T>
    where
        T: DynamicSoundHandle + 'static,
    {
        self.0.iter().filter_map(|sound| match sound {
            KiraPlayingSound::Static(_) => None,
            KiraPlayingSound::Dynamic(dyn_handle) => dyn_handle.as_any().downcast_ref::<T>(),
        })
    }
}

/// This event is used to tell [`KiraPlugin`] to play a sound. Once `KiraPlugin` has consumed the
/// event it will request that kira begins playing it. The sound handle for the playing event will
/// be converted into a [`KiraPlayingSound`] and inserted into a [`KiraPlayingSounds`] (notice the
//plural 's' there) component on the entity / specified in the event. This allows the sound to be
//stopped or modified later by querying for
/// `KiraPlayingSounds` in a system.
///
/// [`KiraPlugin`]: crate::KiraPlugin
#[derive(Event)]
pub struct KiraPlaySoundEvent {
    /// The entity that the playing sound should be associated with via the `KiraPlayingSounds`
    /// component.
    pub(super) entity: Entity,
    // The entity to look up the TrackHandle for the sound.
    // If this is `None`, the sound will be played on the default track.
    // If set the entity must have a `KiraTrackHandle` component.
    pub(super) track_entity: Option<Entity>,
    /// The sound to play.
    pub(super) sound: Box<dyn KiraPlayable>,
}

impl KiraPlaySoundEvent {
    pub fn new(entity: Entity, track_entity: Option<Entity>, sound: impl KiraPlayable) -> Self {
        Self {
            entity,
            track_entity,
            sound: Box::new(sound),
        }
    }
}

impl Debug for KiraPlayingSounds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KiraActiveSounds")
            .field("len", &self.0.len())
            .finish()
    }
}

pub(super) fn do_play_sys(
    mut commands: Commands,
    mut kira: NonSendMut<KiraContext>,
    mut query: Query<(Entity, Option<&mut KiraPlayingSounds>)>,
    mut track_query: Query<&mut KiraTrackHandle>,
    mut ev_play: ResMut<Events<KiraPlaySoundEvent>>,
) -> Result<(), BevyError> {
    for event in ev_play.drain() {
        let mut opt_track = if let Some(track_entity) = event.track_entity {
            let res = track_query.get_mut(track_entity)?;
            Some(res)
        } else {
            None
        };
        let sound_handle = match kira.play(event.sound, opt_track.as_deref_mut()) {
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
                        .insert(KiraPlayingSounds(vec![sound_handle]));
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
    Ok(())
}

pub(super) fn cleanup_inactive_sounds_sys(
    mut commands: Commands,
    mut query: Query<(Entity, &mut KiraPlayingSounds)>,
) {
    for (eid, mut sounds) in query.iter_mut() {
        // first check for at least one stopped sound before deref mut to avoid spurious change
        // notifications notification. This is not yet profiled so may be a premature optimization.
        // note that `any` is short-circuiting so we don't need to worry about the cost iterating
        // over every sound.
        let needs_cleanup = sounds.0.iter().any(|sound| match &sound {
            KiraPlayingSound::Static(sound) => sound.state() == PlaybackState::Stopped,
            KiraPlayingSound::Dynamic(sound) => sound.state() == PlaybackState::Stopped,
        });

        if needs_cleanup {
            sounds.0.retain(|sound| match &sound {
                KiraPlayingSound::Static(sound) => sound.state() != PlaybackState::Stopped,
                KiraPlayingSound::Dynamic(sound) => sound.state() != PlaybackState::Stopped,
            });
        }
        if sounds.0.is_empty() {
            commands.entity(eid).remove::<KiraPlayingSounds>();
        }
    }
}
