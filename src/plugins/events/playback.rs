use std::fmt::Debug;
use std::fmt::Formatter;

use bevy::prelude::error;
use bevy::prelude::EventReader;
use bevy::prelude::NonSendMut;
use bevy::prelude::{Commands, Component, Entity, Query};
use bevy::reflect::Reflect;
use kira::sound::static_sound::PlaybackState;
use kira::sound::SoundData;

pub use crate::static_sound_loader::{KiraStaticSoundAsset, StaticSoundFileLoader};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};

use crate::KiraContext;

#[derive(Component, Default, Reflect)]
pub struct KiraActiveSounds(#[reflect(ignore)] pub Vec<StaticSoundHandle>);

pub struct KiraPlaySoundEvent<D: SoundData = StaticSoundData> {
    pub(super) entity: Entity,
    pub(super) sound: D,
}

impl KiraPlaySoundEvent {
    pub fn new(entity: Entity, sound: StaticSoundData) -> Self {
        Self { entity, sound }
    }
}

impl Debug for KiraActiveSounds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KiraActiveSounds")
            .field("len", &self.0.len())
            .finish()
    }
}

pub(super) fn do_play_sys(
    mut commands: Commands,
    mut kira: NonSendMut<KiraContext>,
    mut query: Query<(Entity, Option<&mut KiraActiveSounds>)>,
    mut ev_play: EventReader<KiraPlaySoundEvent>,
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

pub(super) fn cleanup_inactive_sounds_sys(
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
