use std::fmt::Debug;
use std::fmt::Formatter;

use bevy::prelude::error;

use bevy::prelude::Events;
use bevy::prelude::NonSendMut;
use bevy::prelude::ResMut;
use bevy::prelude::{Commands, Component, Entity, Query};
use bevy::reflect::Reflect;
use kira::sound::static_sound::PlaybackState;

pub use crate::static_sound_loader::{KiraStaticSoundAsset, StaticSoundFileLoader};
use crate::KiraPlayable;
use crate::KiraPlayingSound;
use kira::sound::static_sound::StaticSoundHandle;

use crate::KiraContext;

#[derive(Component, Default, Reflect)]
pub struct KiraPlayingSounds(#[reflect(ignore)] pub(crate) Vec<KiraPlayingSound>);

impl KiraPlayingSounds {
    pub fn static_handles(&self) -> impl Iterator<Item = &StaticSoundHandle> {
        self.0.iter().filter_map(|sound| match sound {
            KiraPlayingSound::Static(sound) => Some(sound),
            KiraPlayingSound::Dynamic(_) => None,
        })
    }
    pub fn dynamic_handels<T: 'static>(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(|sound| match sound {
            KiraPlayingSound::Static(_) => None,
            KiraPlayingSound::Dynamic(dyn_handle) => {
                let dyn_any = dyn_handle.as_any();
                if let Some(m) = dyn_any.downcast_ref::<T>() {
                    Some(m)
                } else {
                    None
                }
            }
        })
    }
}

pub struct KiraPlaySoundEvent {
    pub(super) entity: Entity,
    pub(super) sound: Box<dyn KiraPlayable>,
}

impl KiraPlaySoundEvent {
    pub fn new(entity: Entity, sound: impl KiraPlayable) -> Self {
        Self {
            entity,
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
    mut ev_play: ResMut<Events<KiraPlaySoundEvent>>,
) {
    for event in ev_play.drain() {
        let sound_handle = match kira.play_dynamic(event.sound) {
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
